use crate::intercom::NotifierMsg as Message;
use crate::utils::async_msg::{MessageBox, MessageQueue};
use crate::utils::task::TokioServiceInfo;
use chain_impl_mockchain::{fragment::FragmentId, header::HeaderId};
use futures::{select, SinkExt, StreamExt};
use jormungandr_lib::interfaces::notifier::JsonMessage;
use slog::Logger;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, watch};

const MAX_CONNECTIONS_DEFAULT: usize = 255;

// error codes in 4000-4999 are reserved for private use.
// I couldn't find an error code for max connections, so I'll use the first one for now
// maybe using the standard error code for Again is the right thing to do
const MAX_CONNECTIONS_ERROR_CLOSE_CODE: u16 = 4000;
const MAX_CONNECTIONS_ERROR_REASON: &str = "MAX CONNECTIONS reached";

// FIXME: arbitrary value, needs more thinking
const MEMPOOL_MESSAGE_QUEUE_LEN: usize = 128;

pub struct Notifier {
    connection_counter: Arc<AtomicUsize>,
    max_connections: usize,
    tip_sender: Arc<watch::Sender<HeaderId>>,
    tip_receiver: watch::Receiver<HeaderId>,
    block_sender: Arc<broadcast::Sender<HeaderId>>,
    mempool_sender: Arc<broadcast::Sender<MemPoolMessage>>,
}

#[derive(Clone)]
enum MemPoolMessage {
    FragmentAccepted(FragmentId),
    FragmentRejected(FragmentId),
}

#[derive(Clone)]
pub struct NotifierContext(pub MessageBox<Message>);

impl NotifierContext {
    pub async fn new_block_connection(&mut self, ws: warp::ws::WebSocket) {
        &mut self.0.send(Message::NewBlockConnection(ws)).await;
    }

    pub async fn new_mempool_connection(&mut self, ws: warp::ws::WebSocket) {
        &mut self.0.send(Message::NewMempoolConnection(ws)).await;
    }
}

impl Notifier {
    pub fn new(max_connections: Option<usize>, current_tip: HeaderId) -> Notifier {
        let (tip_sender, tip_receiver) = watch::channel(current_tip);
        let (block_sender, _block_receiver) = broadcast::channel(16);
        let (mempool_sender, _mempool_receiver) = broadcast::channel(MEMPOOL_MESSAGE_QUEUE_LEN);

        Notifier {
            connection_counter: Arc::new(AtomicUsize::new(0)),
            max_connections: max_connections.unwrap_or(MAX_CONNECTIONS_DEFAULT),
            tip_sender: Arc::new(tip_sender),
            tip_receiver,
            block_sender: Arc::new(block_sender),
            mempool_sender: Arc::new(mempool_sender),
        }
    }

    pub async fn start(&self, info: TokioServiceInfo, queue: MessageQueue<Message>) {
        let info = Arc::new(info);

        queue
            .for_each(move |input| {
                let tip_sender = Arc::clone(&self.tip_sender);
                let block_sender = Arc::clone(&self.block_sender);
                let mempool_sender = Arc::clone(&self.mempool_sender);
                let logger = info.logger().clone();

                match input {
                    Message::NewBlock(block_id) => {
                        info.spawn("notifier broadcast block", async move {
                            if let Err(_err) = block_sender.send(block_id) {
                                ()
                            }
                        });
                    }
                    Message::NewTip(block_id) => {
                        info.spawn("notifier broadcast new tip", async move {
                            if let Err(_err) = tip_sender.broadcast(block_id) {
                                error!(logger, "notifier failed to broadcast tip {}", block_id);
                            }
                        });
                    }
                    Message::FragmentRejected(fragment_id) => {
                        info.spawn("notifier broadcast fragment rejection", async move {
                            if let Err(_err) =
                                mempool_sender.send(MemPoolMessage::FragmentRejected(fragment_id))
                            {
                                ()
                            }
                        });
                    }
                    Message::FragmentInBlock(fragment_id) => {
                        info.spawn("notifier broadcast fragment accepted", async move {
                            if let Err(_err) =
                                mempool_sender.send(MemPoolMessage::FragmentAccepted(fragment_id))
                            {
                                ()
                            }
                        });
                    }
                    Message::NewBlockConnection(ws) => {
                        let info2 = Arc::clone(&info);

                        let connection_counter = Arc::clone(&self.connection_counter);
                        let max_connections = self.max_connections;
                        let tip_receiver = self.tip_receiver.clone();

                        info.spawn("notifier process new messages", async move {
                            Self::check_max_and_spawn(
                                max_connections,
                                connection_counter,
                                ws,
                                |ws| {
                                    Self::spawn_block_connection(
                                        info2,
                                        ws,
                                        tip_receiver,
                                        block_sender,
                                    )
                                },
                            )
                            .await;
                        });
                    }
                    Message::NewMempoolConnection(ws) => {
                        let info2 = Arc::clone(&info);

                        let connection_counter = Arc::clone(&self.connection_counter);
                        let max_connections = self.max_connections;

                        info.spawn("notifier new mempool subscription", async move {
                            Self::check_max_and_spawn(
                                max_connections,
                                connection_counter,
                                ws,
                                |ws| Self::spawn_mempool_connection(info2, ws, mempool_sender),
                            )
                            .await;
                        });
                    }
                }

                futures::future::ready(())
            })
            .await;
    }

    pub async fn check_max_and_spawn<F: futures::Future<Output = ()>>(
        max_connections: usize,
        connection_counter: Arc<AtomicUsize>,
        mut ws: warp::ws::WebSocket,
        spawn_handler: impl FnOnce(warp::ws::WebSocket) -> F,
    ) {
        let counter = connection_counter.load(Ordering::Acquire);

        if counter < max_connections {
            connection_counter.store(counter + 1, Ordering::Release);

            spawn_handler(ws).await
        } else {
            let close_msg = warp::ws::Message::close_with(
                MAX_CONNECTIONS_ERROR_CLOSE_CODE,
                MAX_CONNECTIONS_ERROR_REASON,
            );
            if ws.send(close_msg).await.is_ok() {
                let _ = ws.close().await;
            }
        }
    }

    async fn spawn_block_connection(
        info: Arc<TokioServiceInfo>,
        mut ws: warp::ws::WebSocket,
        tip_receiver: watch::Receiver<HeaderId>,
        block_sender: Arc<broadcast::Sender<HeaderId>>,
    ) {
        let mut tip_receiver = tip_receiver.fuse();
        let mut block_receiver = block_sender.subscribe().fuse();

        info.spawn("notifier connection", (move || async move {
                loop {
                    select! {
                        msg = tip_receiver.next() => {
                            if let Some(msg) = msg {
                                let warp_msg = warp::ws::Message::text(JsonMessage::NewTip(msg.into()));

                                if let Err(_disconnected) = ws.send(warp_msg).await {
                                    break;
                                }
                            }
                        },
                        msg = block_receiver.next() => {
                            // if this is an Err it means this receiver is lagging, in which case it will
                            // drop messages, I think ignoring that case and continuing with the rest is
                            // fine
                            if let Some(Ok(msg)) = msg {
                                if let Err(_disconnected) = ws.send(warp::ws::Message::text(JsonMessage::NewBlock(msg.into()))).await {
                                    break;
                                }
                            }
                        },
                        complete => break,
                    };
                }

                futures::future::ready(())
            })().await);
    }

    async fn spawn_mempool_connection(
        info: Arc<TokioServiceInfo>,
        mut ws: warp::ws::WebSocket,
        mempool_sender: Arc<broadcast::Sender<MemPoolMessage>>,
    ) {
        let mut mempool_receiver = mempool_sender.subscribe();

        info.spawn(
            "notifier mempool connection",
            async {
                loop {
                    if let Ok(msg) = mempool_receiver.recv().await {
                        let msg: JsonMessage = msg.into();
                        if let Err(_disconnected) = ws.send(warp::ws::Message::text(msg)).await {
                            break;
                        }
                    }
                }

                futures::future::ready(())
            }
            .await,
        );
    }
}

impl Into<JsonMessage> for MemPoolMessage {
    fn into(self) -> JsonMessage {
        match self {
            MemPoolMessage::FragmentAccepted(fragment_id) => {
                JsonMessage::FragmentAccepted(fragment_id.into())
            }
            MemPoolMessage::FragmentRejected(fragment_id) => {
                JsonMessage::FragmentRejected(fragment_id.into())
            }
        }
    }
}
