use crate::{
    service::Service,
    watchdog::{ControlCommand, WatchdogError, WatchdogQuery},
};
use std::ops::{Deref, DerefMut};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub struct NoIntercom;

pub trait IntercomMsg: std::fmt::Debug + 'static {}

pub struct Intercom<T: Service> {
    state: IntercomState<T::Intercom>,
    watchdog_query: WatchdogQuery,
}

enum IntercomState<T> {
    NotConnected,
    Disconnected, // TODO: add reason if any?
    Connected { connection: IntercomSender<T> },
}

pub struct IntercomSender<T>(mpsc::Sender<T>);

pub struct IntercomReceiver<T>(mpsc::Receiver<T>);

impl IntercomMsg for NoIntercom {}

pub fn channel<T: IntercomMsg>() -> (IntercomSender<T>, IntercomReceiver<T>) {
    let (sender, receiver) = mpsc::channel(10);

    (IntercomSender(sender), IntercomReceiver(receiver))
}

impl<T: Service> Intercom<T> {
    pub(crate) fn new(watchdog_query: WatchdogQuery) -> Self {
        Self {
            state: IntercomState::NotConnected,
            watchdog_query,
        }
    }

    /// try to send the given message to the associated service. The command
    /// will attempt to reconnect if needed (if the intercom message has been closed).
    ///
    /// however, there is a 100ms delay before doing a retry. Only one retry
    /// will be perform.
    pub async fn send(&mut self, msg: T::Intercom) -> Result<(), WatchdogError> {
        use tokio::sync::mpsc::error::SendError;

        let mut retry_attempted = false;
        let mut retry = Err(msg);

        while let Err(msg) = retry {
            retry = match &mut self.state {
                IntercomState::Connected { connection } => {
                    connection.send(msg).await.map_err(|SendError(msg)| msg)
                }
                _ => Err(msg),
            };

            if retry.is_err() && retry_attempted {
                return Err(WatchdogError::CannotConnectToService {
                    service_identifier: T::SERVICE_IDENTIFIER,
                    retry_attempted,
                });
            }

            if retry.is_err() {
                retry_attempted = true;
                tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
                self.connect().await?;
            }
        }

        Ok(())
    }

    fn disconnect(&mut self) {
        self.state = IntercomState::Disconnected;
    }

    async fn connect(&mut self) -> Result<(), WatchdogError> {
        // make sure we are disconnected
        self.disconnect();

        let (reply, receiver) = oneshot::channel();

        let command = ControlCommand::Intercom {
            service_identifier: T::SERVICE_IDENTIFIER,
            reply,
        };
        self.watchdog_query.send(command).await;

        match receiver.await {
            Ok(Ok(intercom_sender)) => {
                let tid = intercom_sender.type_id();
                match intercom_sender.downcast_ref::<IntercomSender<T::Intercom>>() {
                    Some(intercom_sender_ref) => {
                        self.state = IntercomState::Connected {
                            connection: intercom_sender_ref.clone(),
                        };
                        Ok(())
                    }
                    None => unreachable!(
                        "cannot downcast the intercom object to {}, {:?}",
                        std::any::type_name::<T::Intercom>(),
                        tid,
                    ),
                }
            }
            Ok(Err(err)) => Err(err),
            Err(err) => {
                // we assume the server will always reply one way or another
                unreachable!(
                    "The watchdog didn't reply to on the intercom query, {:#?}",
                    err
                )
            }
        }
    }
}

impl<T> Clone for IntercomSender<T> {
    fn clone(&self) -> Self {
        IntercomSender(self.0.clone())
    }
}

impl<T> Deref for IntercomSender<T> {
    type Target = mpsc::Sender<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for IntercomSender<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Deref for IntercomReceiver<T> {
    type Target = mpsc::Receiver<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for IntercomReceiver<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
