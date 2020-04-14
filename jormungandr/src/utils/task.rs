//! # Task management
//!
//! Create a task management to leverage the tokio framework
//! in order to more finely organize and control the different
//! modules utilized in jormungandr.
//!

use crate::log;

use slog::Logger;
use tokio_compat::runtime::{self, Runtime, TaskExecutor};

use std::fmt::Debug;
use std::future::Future;
use std::sync::mpsc::{self, Receiver, RecvError, Sender};
use std::time::{Duration, Instant};

/// hold onto the different services created
pub struct Services {
    logger: Logger,
    services: Vec<Service>,
    finish_listener: ServiceFinishListener,
    runtime: Runtime,
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
}

/// the current future service information
///
/// retrieve the name, the up time, the logger and the executor
pub struct TokioServiceInfo {
    name: &'static str,
    up_time: Instant,
    logger: Logger,
    executor: TaskExecutor,
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

impl Services {
    /// create a new set of services
    pub fn new(logger: Logger) -> Self {
        Services {
            logger: logger,
            services: Vec::new(),
            finish_listener: ServiceFinishListener::new(),
            runtime: runtime::Builder::new().build().unwrap(),
        }
    }

    /// Spawn the given Future in a new dedicated runtime
    pub fn spawn_future_std<F, T>(&mut self, name: &'static str, f: F)
    where
        F: FnOnce(TokioServiceInfo) -> T,
        F: Send + 'static,
        T: Future<Output = ()> + Send + 'static,
    {
        let logger = self
            .logger
            .new(o!(crate::log::KEY_TASK => name))
            .into_erased();

        let executor = self.runtime.executor();
        let now = Instant::now();
        let future_service_info = TokioServiceInfo {
            name,
            up_time: now,
            logger: logger.clone(),
            executor,
        };

        let finish_notifier = self.finish_listener.notifier();
        self.runtime.spawn_std(async move {
            f(future_service_info).await;
            info!(logger, "service finished");
            // send the finish notifier if the service finished with an error.
            // this will allow to finish the node with an error code instead
            // of an success error code
            let _ = finish_notifier.sender.send(true);
            // Holds finish notifier, so it's dropped when whole future finishes or is dropped
            std::mem::drop(finish_notifier);
        });

        let task = Service::new(name, now);
        self.services.push(task);
    }

    /// Spawn the given Future in a new dedicated runtime
    pub fn spawn_try_future_std<F, T>(&mut self, name: &'static str, f: F)
    where
        F: FnOnce(TokioServiceInfo) -> T,
        F: Send + 'static,
        T: Future<Output = Result<(), ()>> + Send + 'static,
    {
        let logger = self
            .logger
            .new(o!(crate::log::KEY_TASK => name))
            .into_erased();

        let executor = self.runtime.executor();
        let now = Instant::now();
        let future_service_info = TokioServiceInfo {
            name,
            up_time: now,
            logger: logger.clone(),
            executor,
        };

        let finish_notifier = self.finish_listener.notifier();
        self.runtime.spawn_std(async move {
            let res = f(future_service_info).await;
            let outcome = if res.is_ok() {
                "successfully"
            } else {
                "with error"
            };
            info!(logger, "service finished {}", outcome);

            // send the finish notifier if the service finished with an error.
            // this will allow to finish the node with an error code instead
            // of an success error code
            let _ = finish_notifier.sender.send(res.is_ok());
            // Holds finish notifier, so it's dropped when whole future finishes or is dropped
            std::mem::drop(finish_notifier);
        });

        let task = Service::new(name, now);
        self.services.push(task);
    }

    /// select on all the started services. this function will block until first services returns
    pub fn wait_any_finished(&self) -> Result<bool, RecvError> {
        self.finish_listener.wait_any_finished()
    }

    // Run the task to completion
    pub fn block_on_task_std<F, Fut, T>(&mut self, name: &'static str, f: F) -> T
    where
        F: FnOnce(TokioServiceInfo) -> Fut,
        Fut: Future<Output = T>,
    {
        let logger = self
            .logger
            .new(o!(crate::log::KEY_TASK => name))
            .into_erased();
        let executor = self.runtime.executor();
        let now = Instant::now();
        let future_service_info = TokioServiceInfo {
            name,
            up_time: now,
            logger: logger,
            executor,
        };
        self.runtime.block_on_std(f(future_service_info))
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

    /// Access the service's executor
    #[inline]
    pub fn executor(&self) -> &TaskExecutor {
        &self.executor
    }

    /// access the service's logger
    #[inline]
    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    /// Extract the service's logger, dropping the rest of the
    /// `TokioServiceInfo` instance.
    #[inline]
    pub fn into_logger(self) -> Logger {
        self.logger
    }

    /// spawn a std::future within the service's tokio executor
    pub fn spawn_std<F>(&self, name: &'static str, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let logger = self.logger.clone();
        trace!(logger, "spawning {}", name);
        self.executor.spawn_std(future)
    }

    /// just like spawn_std but instead log an error on Result::Err
    pub fn spawn_failable_std<F, E>(&self, name: &'static str, future: F)
    where
        F: Send + 'static,
        E: Debug,
        F: Future<Output = Result<(), E>>,
    {
        let logger = self.logger.clone();
        trace!(logger, "spawning {}", name);
        self.executor.spawn_std(async move {
            match future.await {
                Ok(()) => trace!(logger, "{} finished successfully", name),
                Err(e) => error!(logger, "{} finished with error", name; "error" => ?e),
            }
        })
    }

    /// just like spawn_std but add a timeout
    pub fn timeout_spawn_std<F>(&self, name: &'static str, timeout: Duration, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let logger = self.logger.clone();
        trace!(logger, "spawning {}", name);
        self.executor.spawn_std(async move {
            match tokio02::time::timeout(timeout, future).await {
                Err(_) => error!(logger, "task {} timedout", name),
                Ok(()) => {}
            };
        })
    }

    /// just like spawn_failable_std but add a timeout
    pub fn timeout_spawn_failable_std<F, E>(&self, name: &'static str, timeout: Duration, future: F)
    where
        F: Send + 'static,
        E: Debug,
        F: Future<Output = Result<(), E>>,
    {
        let logger = self.logger.clone();
        trace!(logger, "spawning {}", name);
        self.executor.spawn_std(async move {
            match tokio02::time::timeout(timeout, future).await {
                Err(_) => error!(logger, "task {} timedout", name),
                Ok(Err(e)) => error!(logger, "task {} finished with error", name; "error" => ?e),
                Ok(Ok(())) => {}
            };
        })
    }

    // Run the closure with the specified period on the executor
    // and execute the resulting closure.
    pub fn run_periodic_std<F, U, E>(&self, name: &'static str, period: Duration, mut f: F)
    where
        F: FnMut() -> U,
        F: Send + 'static,
        E: Debug,
        U: Future<Output = Result<(), E>> + Send + 'static,
    {
        let logger = self.logger.new(o!(log::KEY_SUB_TASK => name));
        self.spawn_std(name, async move {
            let mut interval = tokio02::time::interval(period);
            loop {
                let t_now = Instant::now();
                interval.tick().await;
                let t_last = Instant::now();
                let elapsed = t_last.duration_since(t_now);
                if elapsed > period * 2 {
                    warn!(logger, "periodic task started late"; "period" => ?period, "elapsed" => ?elapsed);
                }
                match f().await {
                    Ok(()) => {
                        trace!(logger, "periodic {} finished successfully", name; "triggered_at" => ?t_now);
                    },
                    Err(e) => {
                        error!(logger, "periodic task failed"; "error" => ?e, "triggered_at" => ?t_now);
                    },
                };
            }
        });
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
    fn new(name: &'static str, now: Instant) -> Self {
        Service { name, up_time: now }
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

struct ServiceFinishListener {
    sender: Sender<bool>,
    receiver: Receiver<bool>,
}

/// Sends notification when dropped
struct ServiceFinishNotifier {
    sender: Sender<bool>,
}

impl ServiceFinishListener {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        ServiceFinishListener { sender, receiver }
    }

    pub fn notifier(&self) -> ServiceFinishNotifier {
        ServiceFinishNotifier {
            sender: self.sender.clone(),
        }
    }

    pub fn wait_any_finished(&self) -> Result<bool, RecvError> {
        self.receiver.recv()
    }
}

impl Drop for ServiceFinishNotifier {
    fn drop(&mut self) {
        let _ = self.sender.send(true);
    }
}
