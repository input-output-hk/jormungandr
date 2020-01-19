mod control_command;
mod monitor;

use self::control_command::ControlCommand;
pub use self::{
    control_command::{ControlHandler, WatchdogQuery},
    monitor::WatchdogMonitor,
};
use crate::{service::ServiceError, ServiceIdentifier};
use std::any::Any;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

/// trait to define the different core services and their
/// associated metadata
///
// TODO: write a proc macro to make it easier
//       to impl this object
pub trait CoreServices: Send + Sync {
    fn stop(&mut self, service_identifier: ServiceIdentifier) -> Result<(), WatchdogError>;
    fn start(
        &mut self,
        service_identifier: ServiceIdentifier,
        watchdog_query: WatchdogQuery,
    ) -> Result<(), WatchdogError>;
    fn intercoms(
        &mut self,
        service_identifier: ServiceIdentifier,
    ) -> Result<Box<dyn Any + Send + 'static>, WatchdogError>;
}

pub struct Watchdog<T> {
    services: T,
    on_drop_send: oneshot::Sender<()>,
}

#[derive(Default)]
pub struct WatchdogBuilder;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WatchdogError {
    #[error("Unknown service {service_identifier}, available services are {possible_values:?}")]
    UnknownService {
        service_identifier: ServiceIdentifier,
        possible_values: &'static [ServiceIdentifier],
    },

    #[error("Cannot start service {service_identifier}: {source}")]
    CannotStartService {
        service_identifier: ServiceIdentifier,
        source: ServiceError,
    },
}

impl WatchdogBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build<T>(&self, services: T) -> WatchdogMonitor
    where
        T: CoreServices + 'static,
    {
        let (sender, receiver) = mpsc::channel(10);
        let (on_drop_send, on_drop_receive) = oneshot::channel();

        let watchdog = Watchdog {
            on_drop_send,
            services,
        };

        let query = WatchdogQuery::new(sender.clone());
        tokio::spawn(async move { watchdog.watchdog(receiver, query).await });

        WatchdogMonitor::new(sender, on_drop_receive)
    }
}

impl<T> Watchdog<T>
where
    T: CoreServices,
{
    async fn watchdog(
        mut self,
        mut cc: mpsc::Receiver<ControlCommand>,
        watchdog_query: WatchdogQuery,
    ) {
        while let Some(command) = cc.recv().await {
            match command {
                ControlCommand::Shutdown | ControlCommand::Kill => {
                    // TODO: for now we assume shutdown and kill are the same
                    //       but on the long run it will need to send a Shutdown
                    //       signal to every services so they can save state and
                    //       release resources properly
                    break;
                }
                ControlCommand::Start {
                    service_identifier,
                    reply,
                } => {
                    if let Err(reply) = reply.send(
                        self.services
                            .start(service_identifier, watchdog_query.clone()),
                    ) {
                        if let Err(err) = reply {
                            dbg!(
                                "Cannot reply to the ControlHandler that the service {} failed to start: {}",
                                service_identifier,
                                err
                            );
                        } else {
                            dbg!(
                                "Cannot reply to the ControlHandler that the service {} started successfully",
                                service_identifier
                            );
                        }
                    }
                }
                ControlCommand::Stop {
                    service_identifier,
                    reply,
                } => {
                    if let Err(reply) = reply.send(self.services.stop(service_identifier)) {
                        if let Err(err) = reply {
                            dbg!(
                                "Cannot reply to the ControlHandler that the service {} failed to stop: {}",
                                service_identifier,
                                err
                            );
                        } else {
                            dbg!(
                                "Cannot reply to the ControlHandler that the service {} stopped successfully",
                                service_identifier
                            );
                        }
                    }
                }
                ControlCommand::Intercom {
                    service_identifier,
                    reply,
                } => {
                    // TODO: surround the operation with a timeout and
                    //       result to success
                    if let Err(reply) = reply.send(self.services.intercoms(service_identifier)) {
                        if let Err(err) = reply {
                            dbg!(
                                "Cannot reply to the ControlHandler that the service {} failed to start: {}",
                                service_identifier,
                                err
                            );
                        } else {
                            dbg!(
                                "Cannot reply to the ControlHandler that the service {} started successfully",
                                service_identifier
                            );
                        }
                    }
                }
            }
        }

        if self.on_drop_send.send(()).is_err() {
            // ignore error for now
        }
    }
}
