//! # Task management
//!
//! Create a task management to leverage the tokio framework
//! in order to more finely organize and control the different
//! modules utilized in jormungandr.
//!

use futures::{prelude::*, stream::FuturesUnordered};
use std::{
    error,
    fmt::Debug,
    future::Future,
    sync::mpsc::Sender,
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::{
    runtime::{Handle, Runtime},
    task::JoinHandle,
};
use tracing::{span, Level, Span};
use tracing_futures::Instrument;

/// hold onto the different services created
pub struct Services {
    services: Vec<Service>,
    finish_listener: FuturesUnordered<JoinHandle<Result<(), Box<dyn error::Error + Send + Sync>>>>,
    runtime: Runtime,
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error(
        "service panicked: {}",
        .0
        .as_ref()
        .map(|reason| reason.as_ref())
        .unwrap_or("could not serialize the panic"),
    )]
    Panic(Option<String>),
    #[error("service future cancelled")]
    Cancelled,
    #[error("service error")]
    Service(#[source] Box<dyn error::Error>),
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
/// retrieve the name, the up time, the tracing span and the handle
pub struct TokioServiceInfo {
    name: &'static str,
    up_time: Instant,
    span: tracing::Span,
    handle: Handle,
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
    pub fn new() -> Self {
        Services {
            services: Vec::new(),
            finish_listener: FuturesUnordered::new(),
            runtime: Runtime::new().unwrap(),
        }
    }

    /// Spawn the given Future in a new dedicated runtime
    pub fn spawn_future<F, T>(&mut self, name: &'static str, f: F)
    where
        F: FnOnce(TokioServiceInfo) -> T,
        F: Send + 'static,
        T: Future<Output = ()> + Send + 'static,
    {
        let handle = self.runtime.handle().clone();
        let now = Instant::now();
        let tracing_span = span!(Level::TRACE, "service", kind = name);
        let future_service_info = TokioServiceInfo {
            name,
            up_time: now,
            span: tracing_span,
            handle,
        };
        let span_parent = future_service_info.span.clone();
        let handle = self.runtime.spawn(
            async move {
                f(future_service_info).await;
                tracing::info!("service `{}` finished", name);
                Ok::<_, std::convert::Infallible>(()).map_err(Into::into)
            }
            .instrument(span!(
                parent: span_parent,
                Level::TRACE,
                "service",
                kind = name
            )),
        );
        self.finish_listener.push(handle);

        let task = Service::new(name, now);
        self.services.push(task);
    }

    /// Spawn the given Future in a new dedicated runtime
    pub fn spawn_try_future<F, T, E>(&mut self, name: &'static str, f: F)
    where
        F: FnOnce(TokioServiceInfo) -> T,
        F: Send + 'static,
        T: Future<Output = Result<(), E>> + Send + 'static,
        E: error::Error + Send + Sync + 'static,
    {
        let handle = self.runtime.handle().clone();
        let now = Instant::now();
        let tracing_span = span!(Level::TRACE, "service", kind = name);

        let future_service_info = TokioServiceInfo {
            name,
            up_time: now,
            span: tracing_span,
            handle,
        };
        let parent_span = future_service_info.span.clone();
        let handle = self.runtime.spawn(
            async move {
                let res = f(future_service_info).await;
                if let Err(err) = &res {
                    tracing::error!(reason = %err.to_string(), "service finished with error");
                } else {
                    tracing::info!("service `{}` finished successfully", name);
                }
                res.map_err(Into::into)
            }
            .instrument(span!(
                parent: parent_span,
                Level::TRACE,
                "service",
                kind = name
            )),
        );
        self.finish_listener.push(handle);

        let task = Service::new(name, now);
        self.services.push(task);
    }

    /// select on all the started services. this function will block until first services returns
    pub fn wait_any_finished(self) -> Result<(), ServiceError> {
        let finish_listener = self.finish_listener;
        let result = self
            .runtime
            .block_on(async move { finish_listener.into_future().await.0 });
        match result {
            // No services were started or some service exited successfully
            None | Some(Ok(Ok(()))) => Ok(()),
            // Error produced by a service
            Some(Ok(Err(service_error))) => Err(ServiceError::Service(service_error)),
            // A service panicked or was cancelled by the environment
            Some(Err(join_error)) => {
                if join_error.is_cancelled() {
                    Err(ServiceError::Cancelled)
                } else if join_error.is_panic() {
                    let desc = join_error.into_panic().downcast_ref::<String>().cloned();
                    Err(ServiceError::Panic(desc))
                } else {
                    unreachable!("JoinError is either Cancelled or Panic")
                }
            }
        }
    }

    // Run the task to completion
    pub fn block_on_task<F, Fut, T>(&mut self, name: &'static str, f: F) -> T
    where
        F: FnOnce(TokioServiceInfo) -> Fut,
        Fut: Future<Output = T>,
    {
        let handle = self.runtime.handle().clone();
        let now = Instant::now();
        let parent_span = span!(Level::TRACE, "service", kind = name);
        let future_service_info = TokioServiceInfo {
            name,
            up_time: now,
            span: parent_span.clone(),
            handle,
        };
        parent_span.in_scope(|| {
            self.runtime
                .block_on(f(future_service_info).instrument(span!(
                    Level::TRACE,
                    "service",
                    kind = name
                )))
        })
    }
}

impl Default for Services {
    fn default() -> Self {
        Self::new()
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

    /// Access the service's handle
    #[inline]
    pub fn runtime_handle(&self) -> &Handle {
        &self.handle
    }

    /// Access the parent service span
    #[inline]
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// spawn a std::future within the service's tokio handle
    pub fn spawn<F>(&self, name: &'static str, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tracing::trace!("service `{}` spawning task `{}`", self.name, name);
        self.handle
            .spawn(future.instrument(span!(parent: &self.span, Level::TRACE, "task", kind = name)));
    }

    /// just like spawn but instead log an error on Result::Err
    pub fn spawn_fallible<F, E>(&self, name: &'static str, future: F)
    where
        F: Send + 'static,
        E: Debug,
        F: Future<Output = Result<(), E>>,
    {
        tracing::trace!("service `{}` spawning task `{}`", self.name, name);
        self.handle.spawn(
            async move {
                match future.await {
                    Ok(()) => tracing::trace!("task {} finished successfully", name),
                    Err(e) => {
                        tracing::error!(reason = ?e, "task {} finished with error", name)
                    }
                }
            }
            .instrument(span!(parent: &self.span, Level::TRACE, "task", kind = name)),
        );
    }

    /// just like spawn but add a timeout
    pub fn timeout_spawn<F>(&self, name: &'static str, timeout: Duration, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tracing::trace!("spawning {}", name);
        self.handle.spawn(
            async move {
                if tokio::time::timeout(timeout, future).await.is_err() {
                    tracing::error!("task {} timed out", name)
                }
            }
            .instrument(span!(parent: &self.span, Level::TRACE, "task", kind = name)),
        );
    }

    /// just like spawn_failable but add a timeout
    pub fn timeout_spawn_fallible<F, E>(&self, name: &'static str, timeout: Duration, future: F)
    where
        F: Send + 'static,
        E: Debug,
        F: Future<Output = Result<(), E>>,
    {
        tracing::trace!("spawning {}", name);
        self.handle.spawn(
            async move {
                match tokio::time::timeout(timeout, future).await {
                    Err(_) => tracing::error!("task {} timed out", name),
                    Ok(Err(e)) => tracing::error!(reason = ?e, "task {} finished with error", name),
                    Ok(Ok(())) => {}
                };
            }
            .instrument(span!(parent: &self.span, Level::TRACE, "task", kind = name)),
        );
    }

    // Run the closure with the specified period on the handle
    // and execute the resulting closure.
    pub fn run_periodic<F, U>(&self, name: &'static str, period: Duration, mut f: F)
    where
        F: FnMut() -> U,
        F: Send + 'static,
        U: Future<Output = ()> + Send + 'static,
    {
        self.spawn(
            name,
            async move {
                let mut interval = tokio::time::interval(period);
                loop {
                    let t_now = Instant::now();
                    interval.tick().await;
                    let t_last = Instant::now();
                    let elapsed = t_last.duration_since(t_now);
                    if elapsed > period * 2 {
                        tracing::warn!(
                            period = ?period,
                            elapsed = ?elapsed,
                            "periodic task `{}` started late", name
                        );
                    }
                    f().await;
                    tracing::trace!(
                        triggered_at = ?t_now,
                        "periodic task `{}` finished successfully",
                        name
                    );
                }
            }
            .instrument(span!(parent: &self.span, Level::TRACE, "task", kind = name)),
        );
    }

    // Run the closure with the specified period on the handle
    // and execute the resulting fallible async closure.
    // If the closure returns an Err, log it.
    pub fn run_periodic_fallible<F, U, E>(&self, name: &'static str, period: Duration, mut f: F)
    where
        F: FnMut() -> U,
        F: Send + 'static,
        E: Debug,
        U: Future<Output = Result<(), E>> + Send + 'static,
    {
        self.spawn(
            name,
            async move {
                let mut interval = tokio::time::interval(period);
                loop {
                    let t_now = Instant::now();
                    interval.tick().await;
                    let t_last = Instant::now();
                    let elapsed = t_last.duration_since(t_now);
                    if elapsed > period * 2 {
                        tracing::warn!(
                            period = ?period,
                            elapsed = ?elapsed,
                            "periodic task `{}` started late", name
                        );
                    }
                    match f().await {
                        Ok(()) => {
                            tracing::trace!(
                                triggered_at = ?t_now,
                                "periodic task `{}` finished successfully",
                                name,
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                triggered_at = ?t_now,
                                error = ?e,
                                "periodic task `{}` failed", name
                            );
                        }
                    };
                }
            }
            .instrument(span!(parent: &self.span, Level::TRACE, "task", kind = name)),
        );
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
