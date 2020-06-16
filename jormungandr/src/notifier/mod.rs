use crate::utils::async_msg::MessageQueue;
use crate::utils::task::TokioServiceInfo;
use chain_impl_mockchain::header::HeaderId;
use futures::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct Notifier {
    next_user_id: Arc<AtomicUsize>,
    clients: Arc<tokio::sync::Mutex<Clients>>,
}

pub enum Message {
    NewBlock(HeaderId),
}

type Clients = std::collections::HashMap<usize, warp::ws::WebSocket>;

impl Notifier {
    pub fn new() -> Notifier {
        Notifier {
            next_user_id: Arc::new(AtomicUsize::new(1)),
            clients: Default::default(),
        }
    }

    pub async fn start(&mut self, info: TokioServiceInfo, queue: MessageQueue<Message>) {
        let clients = self.clients.clone();
        queue
            .for_each(|input| {
                info.spawn(
                    "notifier new input",
                    process_message(clients.clone(), input),
                );
                futures::future::ready(())
            })
            .await;
    }

    pub async fn new_connection(&self, ws: warp::ws::WebSocket) {
        let id = self.next_user_id.fetch_add(1, Ordering::Relaxed);

        let clients = Arc::clone(&self.clients);

        async move {
            let locked_clients = clients.lock();
            locked_clients.await.insert(id, ws);
        }
        .await
    }
}

async fn process_message(clients: Arc<tokio::sync::Mutex<Clients>>, msg: Message) {
    match msg {
        Message::NewBlock(id) => {
            let clients1 = clients;
            let clients2 = Arc::clone(&clients1);

            let disconnected = async move { notify_all(clients1, id).await };

            clean_disconnected(clients2, disconnected.await)
        }
    }
    .await;
}

async fn notify_all(clients: Arc<tokio::sync::Mutex<Clients>>, block: HeaderId) -> Vec<usize> {
    let clients = clients.clone();
    async move {
        let mut disconnected = vec![];
        let mut clients = clients.lock().await;
        for (client_id, channel) in clients.iter_mut() {
            if let Err(_disconnected) = channel
                .send(warp::ws::Message::text(block.to_string()))
                .await
            {
                disconnected.push(client_id.clone())
            }
        }
        disconnected
    }
    .await
}

async fn clean_disconnected(clients: Arc<tokio::sync::Mutex<Clients>>, disconnected: Vec<usize>) {
    async move {
        let mut clients = clients.lock().await;
        for id in disconnected {
            clients.remove(&id);
        }
    }
    .await;
}
