use crate::utils::async_msg::{channel, MessageBox, MessageQueue};
use crate::utils::task::TokioServiceInfo;
use chain_impl_mockchain::header::HeaderId;
use futures::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone)]
pub struct Notifier {
    next_user_id: Arc<AtomicUsize>,
    clients: Arc<tokio::sync::Mutex<Clients>>,
    max_connections: u32,
}

pub enum Message {
    NewBlock(HeaderId),
    NewTip(HeaderId),
}

type Clients = std::collections::HashMap<usize, warp::ws::WebSocket>;

const MAX_CONNECTIONS_DEFAULT: u32 = 3;
const MAX_CONNECTIONS_ERROR_CLOSE_CODE: u16 = 4000;

impl Notifier {
    pub fn new(max_connections: Option<u32>) -> Notifier {
        Notifier {
            next_user_id: Arc::new(AtomicUsize::new(1)),
            clients: Default::default(),
            max_connections: max_connections.unwrap_or(MAX_CONNECTIONS_DEFAULT),
        }
    }

    pub async fn start(&mut self, info: TokioServiceInfo, queue: MessageQueue<Message>) {
        let clients1 = self.clients.clone();
        let clients2 = self.clients.clone();

        // TODO: what limit should I put in there?
        let (deleted_msgbox, deleted_queue) = channel::<usize>(32);

        info.spawn(
            "clean disconnected notifier clients",
            handle_disconnected(clients2.clone(), deleted_queue),
        );

        queue
            .for_each(|input| {
                info.spawn(
                    "notifier send new messages",
                    process_message(clients1.clone(), input, deleted_msgbox.clone()),
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
                "MAX CONNECTIONS reached",
            );
            if ws.send(close_msg).await.is_ok() {
                let _ = ws.close().await;
            }
        }
    }
}

async fn process_message(
    clients: Arc<tokio::sync::Mutex<Clients>>,
    msg: Message,
    mut disconnected: MessageBox<usize>,
) {
    let warp_msg = match msg {
        Message::NewBlock(id) => warp::ws::Message::text(format!("new block: {}", id.to_string())),
        Message::NewTip(id) => warp::ws::Message::text(format!("new tip: {}", id.to_string())),
    };

    let dead = async move { notify_all(clients, warp_msg).await };

    for id in dead.await {
        disconnected
            .send(id)
            .await
            .expect("couldn't push disconnected client id");
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
