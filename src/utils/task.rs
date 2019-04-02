//! # Task management
//!
//! Create a task management to leverage the tokio framework
//! in order to more finely organize and control the different
//! modules utilized in jormungandr.
//!

use crate::log_wrapper::logger::{get_global_logger, update_thread_logger};
use slog::Logger;
use std::{
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, Instant},
};
use tokio_bus::{Bus, BusReader};

/// hold onto the different services created
pub struct Services {
    services: Vec<Service>,
}

/// wrap up a service
///
/// A service will run with its own runtime system. It will be able to
/// (if configured for) spawn new async tasks that will share that same
/// runtime.
pub struct Service {
    /// this is the name of the service task, useful for logging and
    /// following activity of a given task within the app
    name: &'static str,

    /// provides us with information regarding the up time of the Service
    /// this will allow us to monitor if a service has been restarted
    /// without having to follow the log history of the service.
    up_time: Instant,

    /// the tokio Runtime running the service in
    inner: Inner,
}

/// the current thread service information
///
/// retrieve the name, the up time, the logger
pub struct ThreadServiceInfo {
    name: &'static str,
    up_time: Instant,
    logger: Logger,
}

pub struct TaskMessageBox<Msg>(Sender<Msg>);

pub struct TaskBroadcastBox<Msg: Clone + Sync>(Bus<Msg>);

/// Input for the different task with input service
///
/// If `Shutdown` is passed on, it means either there is
/// no more inputs to read (the Senders have been dropped), or the
/// service has been required to shutdown
pub enum Input<Msg> {
    /// the service has been required to shutdown
    Shutdown,
    /// input for the task
    Input(Msg),
}

enum Inner {
    // Tokio { runtime: runtime::Runtime },
    Thread { handler: thread::JoinHandle<()> },
}

impl Services {
    /// create a new set of services
    pub fn new() -> Self {
        Services {
            services: Vec::new(),
        }
    }

    /// spawn a service in a thread. the service will run as long as the
    /// given function does not return. As soon as the function return
    /// the service stop
    ///
    pub fn spawn<F>(&mut self, name: &'static str, f: F)
    where
        F: FnOnce(ThreadServiceInfo) -> (),
        F: Send + 'static,
    {
        let now = Instant::now();
        let thread_service_info = ThreadServiceInfo {
            name: name,
            up_time: now,
            logger: get_global_logger().new(o!("task" => name.to_owned())),
        };

        let handler = thread::Builder::new()
            .name(name.to_owned())
            // .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                info!("starting task: {}", name);
                // TODO: remove the thread logger and utilise the
                //       normal slog function for now
                update_thread_logger(|logger| logger.new(o!("task"=> name.to_string())));
                f(thread_service_info)
            })
            .unwrap_or_else(|err| panic!("Cannot spawn thread {}: {}", name, err));

        let task = Service::new_handler(name, handler, now);
        self.services.push(task);
    }

    /// spawn a service that will be launched for every given inputs
    ///
    /// the service will stop once there is no more input to read: the function
    /// will be called one last time with `Input::Shutdown` and then will return
    ///
    pub fn spawn_with_inputs<F, Msg, B>(
        &mut self,
        name: &'static str,
        b: B,
        f: F,
    ) -> TaskMessageBox<Msg>
    where
        F: Fn(&ThreadServiceInfo, &mut B, Input<Msg>) -> (),
        F: Send + 'static,
        B: Send + 'static,
        Msg: Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<Msg>();

        self.spawn(name, move |info| {
            let mut captured_b = b;
            loop {
                match rx.recv() {
                    Ok(msg) => f(&info, &mut captured_b, Input::Input(msg)),
                    Err(err) => {
                        warn!(
                            "Shutting down service {} (up since {}): {}",
                            name,
                            humantime::format_duration(info.up_time()),
                            err
                        );
                        f(&info, &mut captured_b, Input::Shutdown);
                        break;
                    }
                }
            }
        });

        TaskMessageBox(tx)
    }

    /// join on all the started services. this function will block
    /// until all services return
    ///
    pub fn wait_all(self) {
        for service in self.services {
            match service.inner {
                Inner::Thread { handler } => handler.join().unwrap(),
            }
        }
    }
}

impl ThreadServiceInfo {
    /// get the time this service has been running since
    #[inline]
    pub fn up_time(&self) -> Duration {
        Instant::now().duration_since(self.up_time)
    }

    /// get the name of this Service
    #[inline]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// access the service's logger
    #[inline]
    pub fn logger(&self) -> &Logger {
        &self.logger
    }
}

impl Service {
    /// get the time this service has been running since
    #[inline]
    pub fn up_time(&self) -> Duration {
        Instant::now().duration_since(self.up_time)
    }

    /// get the name of this Service
    #[inline]
    pub fn name(&self) -> &'static str {
        self.name
    }

    #[inline]
    fn new_handler(name: &'static str, handler: thread::JoinHandle<()>, now: Instant) -> Self {
        Service {
            name,
            up_time: now,
            inner: Inner::Thread { handler },
        }
    }
}

impl<Msg> Clone for TaskMessageBox<Msg> {
    fn clone(&self) -> Self {
        TaskMessageBox(self.0.clone())
    }
}

impl<Msg> TaskMessageBox<Msg> {
    pub fn send_to(&self, a: Msg) {
        self.0.send(a).unwrap()
    }
}

impl<Msg: Clone + Sync> TaskBroadcastBox<Msg> {
    pub fn new(len: usize) -> Self {
        TaskBroadcastBox(Bus::new(len))
    }

    pub fn add_rx(&mut self) -> BusReader<Msg> {
        self.0.add_rx()
    }

    pub fn send_broadcast(&mut self, val: Msg) {
        match self.0.try_broadcast(val) {
            Ok(()) => {}
            Err(_) => panic!("broadcast failed, some network tasks may be blocked"),
        }
    }
}
