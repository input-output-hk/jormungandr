use crate::{
    service::{Intercom, StatusReport},
    watchdog::WatchdogError,
    Service, ServiceIdentifier,
};
use std::{any::Any, fmt, future::Future};
use tokio::{
    runtime::Handle,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

#[derive(Debug)]
pub(crate) enum ControlCommand {
    Shutdown,
    Kill,
    Start {
        service_identifier: ServiceIdentifier,
        reply: Reply<Result<(), WatchdogError>>,
    },
    Stop {
        service_identifier: ServiceIdentifier,
        reply: Reply<Result<(), WatchdogError>>,
    },
    Intercom {
        service_identifier: ServiceIdentifier,
        reply: Reply<Result<Box<dyn Any + 'static + Send>, WatchdogError>>,
    },
    Status {
        service_identifier: ServiceIdentifier,
        reply: Reply<Result<StatusReport, WatchdogError>>,
    },
}

#[derive(Debug)]
pub(crate) struct Reply<T>(pub(crate) oneshot::Sender<T>);

#[derive(Clone, Debug)]
pub struct WatchdogQuery {
    sender: mpsc::Sender<ControlCommand>,
    handle: Handle,
}

impl<T> Reply<Result<T, WatchdogError>> {
    pub(crate) fn reply(self, t: Result<T, WatchdogError>) {
        if let Err(reply) = self.0.send(t) {
            if let Err(err) = reply {
                tracing::error!(
                    "Cannot reply to the ControlHandler that the service failed to return status: {}",
                    err
                );
            } else {
                tracing::error!("Cannot reply to the ControlHandler the service's status");
            }
        }
    }
}

impl WatchdogQuery {
    /// This function creates a control handler from a given [`Watchdog`].
    ///
    /// [`Watchdog`]: ./struct.Watchdog.html
    pub(crate) fn new(handle: Handle, sender: mpsc::Sender<ControlCommand>) -> Self {
        Self { sender, handle }
    }

    /// retrieve an intercom object, allows to connect and send messages to
    /// any given services
    pub fn intercom<T: Service>(&self) -> Intercom<T> {
        Intercom::new(self.clone())
    }

    pub(crate) fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.handle.spawn(future)
    }

    pub async fn status<T: Service>(&mut self) -> Result<StatusReport, WatchdogError> {
        let (reply, receiver) = oneshot::channel();
        self.send(ControlCommand::Status {
            service_identifier: T::SERVICE_IDENTIFIER,
            reply: Reply(reply),
        })
        .await;

        match receiver.await {
            Ok(v) => v,
            Err(err) => {
                // we assume the server will always reply one way or another
                unreachable!("The watchdog didn't reply to the status query, {:#?}", err)
            }
        }
    }

    pub(crate) async fn send(&mut self, cc: ControlCommand) {
        if self.sender.send(cc).await.is_err() {
            // ignore the case where the watchdog is already gone
        }
    }
}

/// the watch dog control handler. This is directly linked to the associated
/// [`Watchdog`].
///
/// ## Errors and common issues
///
/// It is impossible to clone the ControlHandler ([LLR-WCI-2]). This is
/// because the control handler allows privileged access to the watchdog
/// control interface. Reducing its usability make sure it is not possible
/// to give control unless actively taken from the monitor prior waiting
/// the watchdog's shutdown.
///
/// ```compile_fail
/// # use jormungandr_watchdog::{WatchdogBuilder, ControlHandler};
/// # let watchdog = WatchdogBuilder::new().build();
///
/// let control_handler = watchdog.control();
///
/// let _ = control_handler.clone(); // impossible
/// ```
///
/// [`Watchdog`]: ./struct.Watchdog.html
/// [`LLR-WCI-2`]: #
pub struct ControlHandler {
    sender: mpsc::Sender<ControlCommand>,
}

impl ControlHandler {
    /// This function creates a control handler from a given [`Watchdog`].
    ///
    /// [`Watchdog`]: ./struct.Watchdog.html
    pub(crate) fn new(sender: mpsc::Sender<ControlCommand>) -> Self {
        Self { sender }
    }

    pub async fn shutdown(&mut self) {
        self.send(ControlCommand::Shutdown).await
    }

    pub async fn kill(&mut self) {
        self.send(ControlCommand::Kill).await
    }

    pub async fn start(
        &mut self,
        service_identifier: ServiceIdentifier,
    ) -> Result<(), WatchdogError> {
        let (reply, receiver) = oneshot::channel();

        let command = ControlCommand::Start {
            service_identifier,
            reply: Reply(reply),
        };
        self.send(command).await;

        match receiver.await {
            Ok(result) => result,
            Err(err) => {
                // we assume the server will always reply one way or another
                unreachable!("The watchdog didn't reply to the start query, {:#?}", err)
            }
        }
    }

    pub async fn stop(
        &mut self,
        service_identifier: ServiceIdentifier,
    ) -> Result<(), WatchdogError> {
        let (reply, receiver) = oneshot::channel();

        let command = ControlCommand::Stop {
            service_identifier,
            reply: Reply(reply),
        };
        self.send(command).await;

        match receiver.await {
            Ok(result) => result,
            Err(err) => {
                // we assume the server will always reply one way or another
                unreachable!("The watchdog didn't reply to the stop query, {:#?}", err)
            }
        }
    }

    pub async fn status<T: Service>(&mut self) -> Result<StatusReport, WatchdogError> {
        let (reply, receiver) = oneshot::channel();
        self.send(ControlCommand::Status {
            service_identifier: T::SERVICE_IDENTIFIER,
            reply: Reply(reply),
        })
        .await;

        match receiver.await {
            Ok(v) => v,
            Err(err) => {
                // we assume the server will always reply one way or another
                unreachable!("The watchdog didn't reply to the status query, {:#?}", err)
            }
        }
    }

    async fn send(&mut self, cc: ControlCommand) {
        if self.sender.send(cc).await.is_err() {
            // ignore the case where the watchdog is already gone
        }
    }
}

impl fmt::Display for ControlCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Shutdown => f.write_str("shutdown"),
            Self::Kill => f.write_str("kill"),
            Self::Start {
                service_identifier, ..
            } => write!(f, "start service '{}'", service_identifier),
            Self::Stop {
                service_identifier, ..
            } => write!(f, "stop service '{}'", service_identifier),
            Self::Status {
                service_identifier, ..
            } => write!(f, "get status of service '{}'", service_identifier),
            Self::Intercom {
                service_identifier, ..
            } => write!(f, "get intercom with service '{}'", service_identifier),
        }
    }
}
