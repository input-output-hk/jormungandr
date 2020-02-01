use crate::{
    service::{Service, Stats},
    watchdog::{ControlCommand, WatchdogError, WatchdogQuery},
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;
use tokio::sync::{
    mpsc::{self, error::SendError},
    oneshot, Mutex,
};

#[derive(Debug)]
pub struct NoIntercom;

pub trait IntercomMsg: std::fmt::Debug + 'static {}

pub struct Intercom<T: Service> {
    state: IntercomState<T::Intercom>,
    watchdog_query: WatchdogQuery,
}

enum IntercomState<T> {
    NotConnected,
    Disconnected,
    Connected { connection: IntercomSender<T> },
}

pub struct IntercomStats {
    sent_counter: Arc<AtomicU64>,
    received_counter: Arc<AtomicU64>,
    stats: Arc<Mutex<Stats>>,
}

pub struct IntercomSender<T> {
    sender: mpsc::Sender<(Instant, T)>,
    sent_counter: Arc<AtomicU64>,
}

pub struct IntercomReceiver<T> {
    receiver: mpsc::Receiver<(Instant, T)>,
    received_counter: Arc<AtomicU64>,
    stats: Arc<Mutex<Stats>>,
}

impl IntercomMsg for NoIntercom {}

#[derive(Debug, Clone, Copy)]
pub struct IntercomStatus {
    /// number of messages that has been sent through the intercom
    pub number_sent: u64,
    /// the number of messages that has been actually read from
    /// the intercom
    pub number_received: u64,
    /// number of opened connection to the service
    pub number_connections: usize,
    /// mean to the time it gets between when a message is sent and
    /// when it is actually received by the Service.
    pub processing_speed_mean: f64,
    pub processing_speed_variance: f64,
    pub processing_speed_standard_derivation: f64,
}

pub fn channel<T: IntercomMsg>() -> (IntercomSender<T>, IntercomReceiver<T>, IntercomStats) {
    let (sender, receiver) = mpsc::channel(10);

    let sent_counter = Arc::new(AtomicU64::new(0));
    let received_counter = Arc::new(AtomicU64::new(0));
    let stats = Arc::new(Mutex::new(Stats::new()));

    (
        IntercomSender {
            sender,
            sent_counter: Arc::clone(&sent_counter),
        },
        IntercomReceiver {
            receiver,
            received_counter: Arc::clone(&received_counter),
            stats: Arc::clone(&stats),
        },
        IntercomStats {
            sent_counter,
            received_counter,
            stats,
        },
    )
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
        let mut retry_attempted = false;
        let mut retry = Err(msg);

        while let Err(msg) = retry {
            retry = match &mut self.state {
                IntercomState::Connected { connection } => {
                    let r = connection.send(msg).await.map_err(|SendError(msg)| msg);
                    connection.sent_counter.fetch_add(1, Ordering::SeqCst);
                    r
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

impl<T> IntercomReceiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        let r = self.receiver.recv().await;

        if let Some((instant, t)) = r {
            self.received_counter.fetch_add(1, Ordering::SeqCst);
            let f = instant.elapsed().as_secs_f64();

            {
                let mut stats = self.stats.lock().await;
                stats.push(f);
            }

            Some(t)
        } else {
            None
        }
    }
}

impl IntercomStats {
    pub async fn status(&self) -> IntercomStatus {
        let stats = self.stats.lock().await;

        IntercomStatus {
            number_sent: self.sent(),
            number_received: self.received(),
            number_connections: self.number_connections(),
            processing_speed_mean: stats.mean(),
            processing_speed_variance: stats.variance(),
            processing_speed_standard_derivation: stats.standard_derivation(),
        }
    }

    pub fn received(&self) -> u64 {
        self.received_counter.load(Ordering::SeqCst)
    }

    pub fn sent(&self) -> u64 {
        self.sent_counter.load(Ordering::SeqCst)
    }

    pub fn number_connections(&self) -> usize {
        Arc::strong_count(&self.sent_counter)
    }
}

impl<T> IntercomSender<T> {
    async fn send(&mut self, t: T) -> Result<(), SendError<T>> {
        self.sender
            .send((Instant::now(), t))
            .await
            .map_err(|SendError((_, t))| SendError(t))
    }
}

impl<T> Clone for IntercomSender<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            sent_counter: Arc::clone(&self.sent_counter),
        }
    }
}
