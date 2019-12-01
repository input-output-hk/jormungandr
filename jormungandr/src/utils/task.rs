//! # Task management
//!
//! Create a task management to leverage the tokio framework
//! in order to more finely organize and control the different
//! modules utilized in jormungandr.
//!

use crate::utils::async_msg::{self, MessageBox};
use slog::Logger;
use std::{
    any::Any,
    sync::mpsc::{self, Receiver, RecvError, Sender},
    time::{Duration, Instant},
};
use tokio::prelude::{stream, Future, IntoFuture, Stream};
use tokio::runtime::{self, Runtime, TaskExecutor};

// Limit on the length of a task message queue
const MESSAGE_QUEUE_LEN: usize = 1000;

/// hold onto the different services created
pub struct Services {
    logger: Logger,
    services: Vec<Service>,
    finish_listener: ServiceFinishListener,
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
    #[allow(dead_code)]
    runtime: Option<Runtime>,
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
        }
    }

    /// Spawn the given Future in a new dedicated runtime
    pub fn spawn_future<F, T>(&mut self, name: &'static str, f: F)
    where
        F: FnOnce(TokioServiceInfo) -> T,
        T: Future<Item = (), Error = ()> + Send + 'static,
    {
        let logger = self
            .logger
            .new(o!(crate::log::KEY_TASK => name))
            .into_erased();
        let panic_logger = logger.clone();
        let mut runtime = runtime::Builder::new()
            .keep_alive(None)
            .name_prefix(name)
            .panic_handler(move |error| log_service_panic(&panic_logger, &*error))
            .build()
            .unwrap();

        let executor = runtime.executor();
        let now = Instant::now();
        let future_service_info = TokioServiceInfo {
            name,
            up_time: now,
            logger: logger.clone(),
            executor,
        };

        let finish_notifier = self.finish_listener.notifier();
        let future = f(future_service_info).then(move |res| {
            let outcome = match res {
                Ok(_) => "successfully",
                Err(_) => "with error",
            };
            info!(logger, "service finished {}", outcome);
            // send the finish notifier if the service finished with an error.
            // this will allow to finish the node with an error code instead
            // of an success error code
            let _ = finish_notifier.sender.send(res.is_ok());
            // Holds finish notifier, so it's dropped when whole future finishes or is dropped
            std::mem::drop(finish_notifier);
            Ok(())
        });

        runtime.spawn(future);

        let task = Service::new_runtime(name, runtime, now);
        self.services.push(task);
    }

    /// Spawn a tokio service that will await messages and will be executed
    /// sequentially for every received inputs
    pub fn spawn_future_with_inputs<F, Msg, T>(
        &mut self,
        name: &'static str,
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
        self.spawn_future(name, move |future_service_info| {
            msg_queue
                .map(Input::Input)
                .chain(stream::once(Ok(Input::Shutdown)))
                .for_each(move |input| f(&future_service_info, input))
        });
        msg_box
    }

    /// select on all the started services. this function will block until first services returns
    pub fn wait_any_finished(&self) -> Result<bool, RecvError> {
        self.finish_listener.wait_any_finished()
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

    /// extract the service's logger
    #[inline]
    pub fn into_logger(self) -> Logger {
        self.logger
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
    fn new_handler(name: &'static str, now: Instant) -> Self {
        Service {
            name,
            up_time: now,
            runtime: None,
        }
    }

    #[inline]
    fn new_runtime(name: &'static str, runtime: Runtime, now: Instant) -> Self {
        Service {
            name,
            up_time: now,
            runtime: Some(runtime),
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

    #[allow(dead_code)]
    pub fn wait_all_finished(self) {
        std::mem::drop(self.sender);
        while let Ok(_) = self.receiver.recv() {
            continue;
        }
    }
}

impl Drop for ServiceFinishNotifier {
    fn drop(&mut self) {
        let _ = self.sender.send(true);
    }
}

fn log_service_panic(logger: &Logger, error: &dyn Any) {
    let reason_logger = error
        .downcast_ref::<&str>()
        .map(|reason| logger.new(o!("reason" => *reason)));
    let panic_logger = reason_logger.as_ref().unwrap_or(logger);
    crit!(panic_logger, "Task panicked");
}
