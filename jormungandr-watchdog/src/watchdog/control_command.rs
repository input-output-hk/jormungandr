use crate::{service::Intercom, watchdog::WatchdogError, Service, ServiceIdentifier};
use std::any::Any;
use tokio::sync::{mpsc, oneshot};

pub(crate) enum ControlCommand {
    Shutdown,
    Kill,
    Start {
        service_identifier: ServiceIdentifier,
        reply: oneshot::Sender<Result<(), WatchdogError>>,
    },
    Stop {
        service_identifier: ServiceIdentifier,
        reply: oneshot::Sender<Result<(), WatchdogError>>,
    },
    Intercom {
        service_identifier: ServiceIdentifier,
        reply: oneshot::Sender<Result<Box<dyn Any + 'static + Send>, WatchdogError>>,
    },
}

#[derive(Clone)]
pub struct WatchdogQuery {
    sender: mpsc::Sender<ControlCommand>,
}

impl WatchdogQuery {
    /// This function creates a control handler from a given [`Watchdog`].
    ///
    /// [`Watchdog`]: ./struct.Watchdog.html
    pub(crate) fn new(sender: mpsc::Sender<ControlCommand>) -> Self {
        Self { sender }
    }

    /// retrieve an intercom object, allows to connect and send messages to
    /// any given services
    pub fn intercom<T: Service>(&self) -> Intercom<T> {
        Intercom::new(self.clone())
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
            reply,
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
            reply,
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

    async fn send(&mut self, cc: ControlCommand) {
        if self.sender.send(cc).await.is_err() {
            // ignore the case where the watchdog is already gone
        }
    }
}
