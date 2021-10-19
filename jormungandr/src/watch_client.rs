pub use crate::intercom::WatchMsg as Message;
use crate::{
    blockcfg::HeaderHash,
    blockchain::{Blockchain, Storage},
    intercom::{self, ReplyStream},
    utils::async_msg::MessageQueue,
};
use crate::{
    intercom::ReplyStreamHandle,
    utils::{async_msg::MessageBox, task::TokioServiceInfo},
};
use chain_core::property::Deserialize;
use chain_core::property::{Block as _, Serialize};
use chain_impl_mockchain::header;
use chain_network::grpc::watch::server::WatchService;
use chain_network::{core::watch::server::Watch, grpc::watch::server};
use chain_network::{
    data::{Block, BlockIds, Header},
    error::Code,
};
use futures::Stream;
use futures::{
    stream::{Map, MapErr},
    SinkExt, StreamExt, TryStream, TryStreamExt,
};
use std::{collections::HashSet, sync::Arc};
use tokio::sync::{broadcast, watch, Mutex};
use tokio_stream::wrappers::{BroadcastStream, WatchStream};
use tracing::{instrument, span, Instrument, Level};

#[derive(Clone)]
pub struct WatchClient {
    tip_receiver: watch::Receiver<Header>,
    block_sender: Arc<broadcast::Sender<Block>>,
    request_tx: Arc<tokio::sync::Mutex<MessageBox<RequestMsg>>>,
}

pub struct MessageProcessor {
    tip_sender: Arc<watch::Sender<Header>>,
    block_sender: Arc<broadcast::Sender<Block>>,
    requests: MessageQueue<RequestMsg>,
    storage: Storage,
    blockchain: Blockchain,
}

enum RequestMsg {
    SyncMultiverse {
        from: BlockIds,
        handle: ReplyStreamHandle<Block>,
    },
}

impl MessageProcessor {
    pub async fn start(self, info: TokioServiceInfo, mut queue: MessageQueue<Message>) {
        let span = span!(Level::TRACE, "watch client message processor");

        let storage = self.storage;
        let requests = self.requests;
        let blockchain = self.blockchain.clone();
        info.spawn(
            "watch client",
            async move {
                requests
                    .for_each(|msg| async {
                        match msg {
                            RequestMsg::SyncMultiverse { from, handle } => {
                                let mut sink = handle.start_sending();

                                if let Err(e) =
                                    handle_sync_multiverse(from, &blockchain, &storage, &mut sink)
                                        .await
                                {
                                    let _ = sink.feed(Err(e)).await;
                                }

                                let _ = sink.close().await;
                            }
                        }
                    })
                    .await;
            }
            .instrument(tracing::info_span!(
                parent: span.clone(),
                "received sync multiverse request"
            )),
        );

        while let Some(input) = queue.next().await {
            match input {
                Message::NewBlock(block) => {
                    let block_sender = Arc::clone(&self.block_sender);
                    let block_id = block.id();
                    info.spawn(
                        "notifier broadcast block",
                        async move {
                            if let Err(_err) = block_sender
                                .send(Block::from_bytes(block.serialize_as_vec().unwrap()))
                            {
                                tracing::trace!(
                                    "there are no subscribers to broadcast block {}",
                                    block_id
                                );
                            }
                        }
                        .instrument(tracing::debug_span!(
                            parent: span.clone(),
                            "block propagation message",
                            ?block_id
                        )),
                    );
                }
                Message::NewTip(header) => {
                    let tip_sender = Arc::clone(&self.tip_sender);
                    let tip_id = header.id();
                    info.spawn(
                        "notifier broadcast new tip",
                        async move {
                            if let Err(err) = tip_sender.send(Header::from_bytes(
                                header.serialize_as_vec().unwrap().as_ref(),
                            )) {
                                tracing::debug!(
                                    "notifier failed to broadcast tip {}, {}",
                                    header.id(),
                                    err
                                );
                            }
                        }
                        .instrument(tracing::debug_span!(
                            parent: span.clone(),
                            "tip propagation message",
                            ?tip_id
                        )),
                    );
                }
            }
        }
    }
}

impl WatchClient {
    pub fn new(
        current_tip: header::Header,
        blockchain: Blockchain,
    ) -> (WatchClient, MessageProcessor) {
        let storage = blockchain.storage().clone();
        let (tip_sender, tip_receiver) = watch::channel(Header::from_bytes(
            current_tip.serialize_as_vec().unwrap().as_ref(),
        ));

        let (block_sender, _block_receiver) = broadcast::channel(16);

        let tip_sender = Arc::new(tip_sender);
        let block_sender = Arc::new(block_sender);

        let (request_tx, requests) = crate::utils::async_msg::channel(16);

        let client = WatchClient {
            tip_receiver,
            block_sender: Arc::clone(&block_sender),
            request_tx: Arc::new(Mutex::new(request_tx)),
        };

        let message_processor = MessageProcessor {
            tip_sender,
            block_sender: Arc::clone(&block_sender),
            storage,
            blockchain,
            requests,
        };

        (client, message_processor)
    }

    pub fn into_server(self) -> server::Server<Self> {
        server::Server::new(WatchService::new(self))
    }
}

type SubscriptionTryStream<S> =
    MapErr<S, fn(<S as TryStream>::Error) -> chain_network::error::Error>;
type SubscriptionStream<S> =
    Map<S, fn(<S as Stream>::Item) -> Result<<S as Stream>::Item, chain_network::error::Error>>;

#[tonic::async_trait]
impl Watch for WatchClient {
    type BlockSubscriptionStream = SubscriptionTryStream<BroadcastStream<Block>>;
    type TipSubscriptionStream = SubscriptionStream<WatchStream<Header>>;
    type SyncMultiverseStream = SubscriptionTryStream<ReplyStream<Block, intercom::Error>>;

    #[instrument(skip(self))]
    async fn block_subscription(
        &self,
    ) -> Result<Self::BlockSubscriptionStream, chain_network::error::Error> {
        let block_receiver = BroadcastStream::new(self.block_sender.subscribe());

        // there are two possible errors for the block_receiver.
        // one occurs when there are no more senders, but that won't happen here.
        // the other is when the receiver is lagging.  I'm actually not sure
        // what would be a sensible choice, so I just put some arbitrary error
        // for now
        let live_stream: SubscriptionTryStream<BroadcastStream<Block>> =
            block_receiver.map_err(|e| chain_network::error::Error::new(Code::Internal, e));

        Ok(live_stream)
    }

    #[instrument(skip(self))]
    async fn tip_subscription(
        &self,
    ) -> Result<Self::TipSubscriptionStream, chain_network::error::Error> {
        let tip_receiver: SubscriptionStream<_> = WatchStream::new(self.tip_receiver.clone())
            .map::<Result<Header, chain_network::error::Error>, _>(Ok);

        Ok(tip_receiver)
    }

    #[instrument(skip(self))]
    async fn sync_multiverse(
        &self,
        from: BlockIds,
    ) -> Result<Self::SyncMultiverseStream, chain_network::error::Error> {
        let (handle, future) = intercom::stream_reply(32);

        self.request_tx
            .lock()
            .await
            .send(RequestMsg::SyncMultiverse { from, handle })
            .await
            .map_err(|e| chain_network::error::Error::new(Code::Unavailable, e))?;

        let stream = future
            .await
            .map_err(|e| chain_network::error::Error::new(Code::Internal, e))?;

        Ok(stream.map_err(|e| chain_network::error::Error::new(Code::Internal, e)))
    }
}

async fn handle_sync_multiverse(
    checkpoints: BlockIds,
    blockchain: &Blockchain,
    storage: &Storage,
    sink: &mut intercom::ReplyStreamSink<Block>,
) -> Result<(), intercom::Error> {
    let mut checkpoints = checkpoints
        .iter()
        .map(|id| HeaderHash::deserialize(id.as_bytes()).map_err(intercom::Error::invalid_argument))
        .collect::<Result<HashSet<_>, _>>()?;

    let branches = blockchain.branches().branches().await;
    let mut previous_branch = None;
    let block0 = blockchain.block0();
    for next_branch in branches {
        let head_id = next_branch.header().id();

        if let Some(prev) = previous_branch.replace(head_id) {
            let lca = storage.find_common_ancestor(prev, head_id).unwrap();

            checkpoints.insert(lca);
        }

        let ancestor = storage
            // TODO: why does find_closest_ancestor need to own the
            // Vec?
            // it could be just impl Iterator I think
            .find_closest_ancestor(checkpoints.iter().cloned().collect(), head_id)
            .map_err(intercom::Error::failed)?
            .map(|ancestor| ancestor.header_hash)
            .unwrap_or(*block0);

        checkpoints.remove(&ancestor);

        // when calling stream_from_to, the from argument is not streamed, this is good for most
        // cases, but if the client is bootstrapping we need to send the block0 too
        if &ancestor == block0 {
            tracing::trace!("streaming block0");

            let block0_body = storage.get(*block0).unwrap().unwrap();
            sink.send(Ok(chain_network::data::Block::from_bytes(
                block0_body.serialize_as_vec().unwrap(),
            )))
            .await
            .map_err(intercom::Error::failed)?;
        }

        tracing::trace!("streaming blocks from {:?} to {:?}", ancestor, head_id);

        let stream = storage
            .stream_from_to(ancestor, head_id)
            .map_err(intercom::Error::failed)?
            .fuse();

        futures::pin_mut!(stream);

        while let Some(block) = stream.next().await {
            let block = block?;

            sink.send(Ok(chain_network::data::Block::from_bytes(
                block.serialize_as_vec().unwrap(),
            )))
            .await
            .map_err(intercom::Error::failed)?;
        }
    }

    Ok(())
}
