pub use crate::intercom::NotifierMsg as Message;
use crate::{
    blockchain::Storage,
    intercom::{self, ReplyStream},
    utils::async_msg::MessageQueue,
};
use crate::{
    intercom::ReplyStreamHandle,
    utils::{async_msg::MessageBox, task::TokioServiceInfo},
};
use chain_core::property::Serialize;
use chain_impl_mockchain::header::HeaderId;
use chain_watch::{
    mempool_event,
    subscription_service_server::{self, SubscriptionServiceServer},
    Block, BlockId, BlockSubscriptionRequest, MempoolEvent, MempoolFragmentInABlock,
    MempoolFragmentInserted, MempoolFragmentRejected, MempoolSubscriptionRequest,
    SyncMultiverseRequest, TipSubscriptionRequest,
};
use futures::Stream;
use futures::{
    stream::{Map, MapErr},
    SinkExt, StreamExt, TryStream, TryStreamExt,
};
use jormungandr_lib::interfaces::FragmentStatus;
use std::sync::Arc;
use tokio::sync::{broadcast, watch, Mutex};
use tokio_stream::wrappers::{BroadcastStream, WatchStream};
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct Notifier {
    tip_receiver: watch::Receiver<BlockId>,
    block_sender: Arc<broadcast::Sender<Block>>,
    mempool_sender: Arc<broadcast::Sender<MempoolEvent>>,
    request_tx: Arc<tokio::sync::Mutex<MessageBox<RequestMsg>>>,
}

pub struct MessageProcessor {
    tip_sender: Arc<watch::Sender<BlockId>>,
    block_sender: Arc<broadcast::Sender<Block>>,
    mempool_sender: Arc<broadcast::Sender<MempoolEvent>>,
    requests: MessageQueue<RequestMsg>,
    storage: Storage,
}

enum RequestMsg {
    PullBlocks {
        from: u32,
        handle: ReplyStreamHandle<Block>,
    },
}

impl MessageProcessor {
    pub async fn start(self, info: TokioServiceInfo, mut queue: MessageQueue<Message>) {
        let storage = self.storage;
        let requests = self.requests;
        info.spawn("notifier client", async move {
            let storage = storage.clone();
            requests
                .for_each(|msg| async {
                    match msg {
                        RequestMsg::PullBlocks { from, handle } => {
                            let mut from = from;
                            let mut sink = handle.start_sending();

                            loop {
                                tracing::debug!("sending block with chainlength");
                                let blocks = storage.get_blocks_by_chain_length(from).unwrap();

                                for block in &blocks {
                                    let _ = sink
                                        .feed(Ok(Block {
                                            content: block.serialize_as_vec().unwrap(),
                                        }))
                                        .await;
                                }

                                from += 1;

                                if blocks.is_empty() {
                                    break;
                                }
                            }
                        }
                    }
                })
                .await;
        });

        while let Some(input) = queue.next().await {
            match input {
                Message::NewBlock(block) => {
                    let block_sender = Arc::clone(&self.block_sender);
                    info.spawn("notifier broadcast block", async move {
                        if let Err(_err) = block_sender.send(Block {
                            content: block.serialize_as_vec().unwrap(),
                        }) {}
                    });
                }
                Message::NewTip(block_id) => {
                    let tip_sender = Arc::clone(&self.tip_sender);
                    info.spawn("notifier broadcast new tip", async move {
                        if let Err(_err) = tip_sender.send(BlockId {
                            content: block_id.serialize_as_vec().unwrap(),
                        }) {
                            tracing::error!("notifier failed to broadcast tip {}", block_id);
                        }
                    });
                }
                Message::FragmentLog(fragment_id, status) => {
                    let mempool_sender = Arc::clone(&self.mempool_sender);
                    info.spawn("notifier broadcast mempool update", async move {
                        let event = match status {
                            FragmentStatus::Pending => {
                                mempool_event::Event::Inserted(MempoolFragmentInserted {})
                            }
                            FragmentStatus::Rejected { reason } => {
                                mempool_event::Event::Rejected(MempoolFragmentRejected { reason })
                            }
                            FragmentStatus::InABlock { block, .. } => {
                                mempool_event::Event::InABlock(MempoolFragmentInABlock {
                                    block: Some(BlockId {
                                        content: block.into_hash().serialize_as_vec().unwrap(),
                                    }),
                                })
                            }
                        };

                        if let Err(_err) = mempool_sender.send(MempoolEvent {
                            fragment_id: fragment_id.serialize_as_vec().unwrap(),
                            event: Some(event),
                        }) {}
                    });
                }
            }
        }
    }
}

impl Notifier {
    pub fn new(current_tip: HeaderId, storage: Storage) -> (Notifier, MessageProcessor) {
        let (tip_sender, tip_receiver) = watch::channel(BlockId {
            content: current_tip.serialize_as_vec().unwrap(),
        });
        let (block_sender, _block_receiver) = broadcast::channel(16);
        let (mempool_sender, _mempool_receiver) = broadcast::channel(16);

        let tip_sender = Arc::new(tip_sender);
        let block_sender = Arc::new(block_sender);
        let mempool_sender = Arc::new(mempool_sender);

        let (request_tx, requests) = crate::utils::async_msg::channel(16);

        let notifier = Notifier {
            tip_receiver,
            block_sender: Arc::clone(&block_sender),
            mempool_sender: Arc::clone(&mempool_sender),
            request_tx: Arc::new(Mutex::new(request_tx)),
        };

        let message_processor = MessageProcessor {
            tip_sender,
            block_sender: Arc::clone(&block_sender),
            mempool_sender: Arc::clone(&mempool_sender),
            storage,
            requests,
        };

        (notifier, message_processor)
    }

    pub fn into_server(self) -> SubscriptionServiceServer<Self> {
        SubscriptionServiceServer::new(self)
    }
}

type SubscriptionTryStream<S> = MapErr<S, fn(<S as TryStream>::Error) -> Status>;
type SubscriptionStream<S> = Map<S, fn(<S as Stream>::Item) -> Result<<S as Stream>::Item, Status>>;

#[tonic::async_trait]
impl subscription_service_server::SubscriptionService for Notifier {
    type BlockSubscriptionStream = SubscriptionTryStream<BroadcastStream<Block>>;
    type TipSubscriptionStream = SubscriptionStream<WatchStream<BlockId>>;
    type MempoolSubscriptionStream = SubscriptionTryStream<BroadcastStream<MempoolEvent>>;
    type SyncMultiverseStream = SubscriptionTryStream<ReplyStream<Block, intercom::Error>>;

    async fn block_subscription(
        &self,
        _request: Request<BlockSubscriptionRequest>,
    ) -> Result<Response<Self::BlockSubscriptionStream>, Status> {
        let block_receiver = BroadcastStream::new(self.block_sender.subscribe());

        // there are two possible errors for the block_receiver.
        // one occurs when there are no more senders, but that won't happen here.
        // the other is when the receiver is lagging.  I'm actually not sure
        // what would be a sensible choice, so I just put some arbitrary error
        // for now
        let live_stream: SubscriptionTryStream<BroadcastStream<Block>> =
            block_receiver.map_err(|_e| Status::deadline_exceeded("some updates were dropped"));

        Ok(Response::new(live_stream))
    }

    async fn tip_subscription(
        &self,
        _request: Request<TipSubscriptionRequest>,
    ) -> Result<Response<Self::TipSubscriptionStream>, Status> {
        let tip_receiver: SubscriptionStream<_> =
            WatchStream::new(self.tip_receiver.clone()).map::<Result<BlockId, Status>, _>(Ok);

        Ok(Response::new(tip_receiver))
    }

    async fn mempool_subscription(
        &self,
        _request: Request<MempoolSubscriptionRequest>,
    ) -> Result<Response<Self::MempoolSubscriptionStream>, Status> {
        let mempool_receiver = BroadcastStream::new(self.mempool_sender.subscribe());

        // see comment in `block_subscription`
        Ok(Response::new(mempool_receiver.map_err(|_e| {
            Status::deadline_exceeded("some updates were dropped")
        })))
    }

    async fn sync_multiverse(
        &self,
        request: Request<SyncMultiverseRequest>,
    ) -> Result<Response<Self::SyncMultiverseStream>, Status> {
        let from = request.get_ref().from;

        let (handle, future) = intercom::stream_reply(32);

        self.request_tx
            .lock()
            .await
            .send(RequestMsg::PullBlocks { from, handle })
            .await
            .map_err(|_| Status::internal("can't process request"))?;

        let stream = future
            .await
            .unwrap()
            .map_err((|_| Status::internal("error")) as fn(intercom::Error) -> Status);

        Ok(Response::new(stream))
    }
}
