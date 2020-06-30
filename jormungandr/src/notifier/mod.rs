use crate::intercom::NotifierMsg as Message;
use crate::utils::async_msg::{channel, MessageBox, MessageQueue};
use crate::utils::task::TokioServiceInfo;
use chain_impl_mockchain::header::HeaderId;
use futures::{SinkExt, StreamExt};
use serde::{Serialize, Serializer};
use slog::Logger;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const MAX_CONNECTIONS_DEFAULT: usize = 255;

// error codes in 4000-4999 are reserved for private use.
// I couldn't find an error code for max connections, so I'll use the first one for now
// maybe using the standard error code for Again is the right thing to do
const MAX_CONNECTIONS_ERROR_CLOSE_CODE: u16 = 4000;
const MAX_CONNECTIONS_ERROR_REASON: &str = "MAX CONNECTIONS reached";

#[derive(Clone)]
pub struct Notifier {
    next_user_id: Arc<AtomicUsize>,
    clients: Arc<tokio::sync::Mutex<Clients>>,
    max_connections: usize,
}

#[derive(Serialize)]
enum JsonMessage {
    #[serde(serialize_with = "to_hex")]
    NewBlock(HeaderId),
    #[serde(serialize_with = "to_hex")]
    NewTip(HeaderId),
}

type Clients = std::collections::HashMap<usize, warp::ws::WebSocket>;

impl Notifier {
    pub fn new(max_connections: Option<usize>) -> Notifier {
        Notifier {
            next_user_id: Arc::new(AtomicUsize::new(1)),
            clients: Default::default(),
            max_connections: max_connections.unwrap_or(MAX_CONNECTIONS_DEFAULT),
        }
    }

    pub async fn start(&mut self, info: TokioServiceInfo, queue: MessageQueue<Message>) {
        let clients1 = self.clients.clone();
        let clients2 = self.clients.clone();
        let logger = info.logger();

        let (deleted_msgbox, deleted_queue) = channel::<usize>(32);

        // TODO: it may be better to have a task that runs periodically instead of
        // when a sender is detected to be disconected, but that would require
        // reading the sockets besides from writing to them
        info.spawn(
            "clean disconnected notifier clients",
            handle_disconnected(clients2.clone(), deleted_queue),
        );

        queue
            .for_each(|input| {
                info.spawn(
                    "notifier send new messages",
                    process_message(
                        logger.clone(),
                        clients1.clone(),
                        input,
                        deleted_msgbox.clone(),
                    ),
                );
                futures::future::ready(())
            })
            .await;
    }

    pub async fn new_connection(&self, ws: warp::ws::WebSocket) {
        let id = self.next_user_id.fetch_add(1, Ordering::Relaxed);

        let clients = Arc::clone(&self.clients);

        let rejected = async move {
            let mut locked_clients = clients.lock().await;
            if locked_clients.len() < (self.max_connections as usize) {
                locked_clients.insert(id, ws);
                None
            } else {
                Some(ws)
            }
        }
        .await;

        if let Some(mut ws) = rejected {
            let close_msg = warp::ws::Message::close_with(
                MAX_CONNECTIONS_ERROR_CLOSE_CODE,
                MAX_CONNECTIONS_ERROR_REASON,
            );
            if ws.send(close_msg).await.is_ok() {
                let _ = ws.close().await;
            }
        }
    }
}

async fn process_message(
    logger: Logger,
    clients: Arc<tokio::sync::Mutex<Clients>>,
    msg: Message,
    mut disconnected: MessageBox<usize>,
) {
    let warp_msg = JsonMessage::from(msg).into();
    let dead = async move { notify_all(clients, warp_msg).await };

    for id in dead.await {
        disconnected.send(id).await.unwrap_or_else(|err| {
            error!(
                logger,
                "notifier error when adding id to disconnected: {}", err
            );
        });
    }
}

async fn notify_all(
    clients: Arc<tokio::sync::Mutex<Clients>>,
    msg: warp::ws::Message,
) -> Vec<usize> {
    let clients = clients.clone();
    async move {
        let mut disconnected = vec![];
        let mut clients = clients.lock().await;
        for (client_id, channel) in clients.iter_mut() {
            if let Err(_disconnected) = channel.send(msg.clone()).await {
                disconnected.push(client_id.clone())
            }
        }
        disconnected
    }
    .await
}

async fn handle_disconnected(
    clients: Arc<tokio::sync::Mutex<Clients>>,
    disconnected: MessageQueue<usize>,
) {
    async move {
        let clients2 = Arc::clone(&clients);
        disconnected
            .for_each(|id| {
                let clients_handle = Arc::clone(&clients2);
                async move {
                    let mut locked_clients = clients_handle.lock().await;
                    locked_clients.remove(&id);
                }
            })
            .await;
    }
    .await;
}

impl From<Message> for JsonMessage {
    fn from(msg: Message) -> JsonMessage {
        match msg {
            Message::NewBlock(inner) => JsonMessage::NewBlock(inner),
            Message::NewTip(inner) => JsonMessage::NewTip(inner),
        }
    }
}

fn to_hex<S>(key: &HeaderId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&key.to_string())
}

impl Into<warp::ws::Message> for JsonMessage {
    fn into(self) -> warp::ws::Message {
        warp::ws::Message::text(serde_json::to_string(&self).unwrap())
    }
}
