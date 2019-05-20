//! # Task management
//!
//! Create a task management to leverage the tokio framework
//! in order to more finely organize and control the different
//! modules utilized in jormungandr.
//!

use crate::utils::async_msg::{self, MessageBox};
use slog::Logger;
use std::{
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, Instant},
};
use tokio::prelude::*;
use tokio::runtime;

// Limit on the length of a task message queue
const MESSAGE_QUEUE_LEN: usize = 1000;

// Stack size of a task thread
const TASK_STACK_SIZE: usize = 2 * 1024 * 1024;

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

/// the current future service information
///
/// retrieve the name, the up time, the logger and the executor
pub struct TokioServiceInfo {
    name: &'static str,
    up_time: Instant,
    logger: Logger,
    executor: runtime::TaskExecutor,
}

pub struct TaskMessageBox<Msg>(Sender<Msg>);

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
    Tokio { runtime: runtime::Runtime },
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
    pub fn spawn<F>(&mut self, name: &'static str, logger: &Logger, f: F)
    where
        F: FnOnce(ThreadServiceInfo) -> (),
        F: Send + 'static,
    {
        let now = Instant::now();
        let thread_service_info = ThreadServiceInfo {
            name: name,
            up_time: now,
            logger: logger.new(o!("task" => name)).into_erased(),
        };

        let handler = thread::Builder::new()
            .name(name.to_owned())
            // .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                slog::info!(thread_service_info.logger, "starting task: {}", name);
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
    pub fn spawn_with_inputs<F, Msg>(
        &mut self,
        name: &'static str,
        logger: &Logger,
        mut f: F,
    ) -> TaskMessageBox<Msg>
    where
        F: FnMut(&ThreadServiceInfo, Input<Msg>) -> (),
        F: Send + 'static,
        Msg: Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<Msg>();

        self.spawn(name, logger, move |info| loop {
            match rx.recv() {
                Ok(msg) => f(&info, Input::Input(msg)),
                Err(err) => {
                    slog::warn!(
                        info.logger,
                        "Shutting down service {} (up since {}): {}",
                        name,
                        humantime::format_duration(info.up_time()),
                        err
                    );
                    f(&info, Input::Shutdown);
                    break;
                }
            }
        });

        TaskMessageBox(tx)
    }

    /// Spawn the given Future in a new dedicated runtime
    ///
    /// * utilising one thread only;
    /// * 2MiB stack size max
    pub fn spawn_future<F, T>(&mut self, name: &'static str, logger: &Logger, f: F)
    where
        F: FnOnce(TokioServiceInfo) -> T,
        T: Future<Item = (), Error = ()> + Send + 'static,
    {
        let mut runtime = runtime::Builder::new()
            .keep_alive(None)
            .core_threads(1)
            .blocking_threads(1)
            .stack_size(TASK_STACK_SIZE)
            .name_prefix(name)
            .build()
            .unwrap();

        let executor = runtime.executor();

        let now = Instant::now();
        let future_service_info = TokioServiceInfo {
            name: name,
            up_time: now,
            logger: logger.new(o!("task" => name)).into_erased(),
            executor: executor,
        };

        runtime.spawn(f(future_service_info));

        let task = Service::new_runtime(name, runtime, now);
        self.services.push(task);
    }

    /// Spawn a tokio service that will await messages and will be executed
    /// sequentially for every received inputs
    ///
    /// * utilising one thread only;
    /// * 2MiB stack size max
    ///
    pub fn spawn_future_with_inputs<F, Msg, T>(
        &mut self,
        name: &'static str,
        logger: &Logger,
        mut f: F,
    ) -> MessageBox<Msg>
    where
        F: FnMut(&TokioServiceInfo, Input<Msg>) -> T,
        F: Send + 'static,
        Msg: Send + 'static,
        T: IntoFuture<Item = (), Error = ()> + Send + 'static,
        <T as futures::IntoFuture>::Future: Send,
    {
        let (msg_box, msg_queue) = async_msg::channel(MESSAGE_QUEUE_LEN);
        self.spawn_future(name, logger, move |future_service_info| {
            msg_queue
                .map(Input::Input)
                .chain(stream::once(Ok(Input::Shutdown)))
                .for_each(move |input| f(&future_service_info, input))
        });
        msg_box
    }

    /// join on all the started services. this function will block
    /// until all services return
    ///
    pub fn wait_all(self) {
        for service in self.services {
            match service.inner {
                Inner::Thread { handler } => handler.join().unwrap(),
                Inner::Tokio { runtime } => runtime.shutdown_on_idle().wait().unwrap(),
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

impl TokioServiceInfo {
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

    /// spawn a future within the service's tokio executor
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Item = (), Error = ()> + Send + 'static,
    {
        self.executor.spawn(future)
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

    #[inline]
    fn new_runtime(name: &'static str, runtime: runtime::Runtime, now: Instant) -> Self {
        Service {
            name,
            up_time: now,
            inner: Inner::Tokio { runtime },
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
