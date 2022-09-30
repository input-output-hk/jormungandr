use super::{
    buffer_sizes,
    convert::{self, Decode, Encode, ResponseStream},
    p2p::{
        comm::{BlockEventSubscription, FragmentSubscription, GossipSubscription},
        Address,
    },
    subscription, Channels, GlobalStateR,
};
use crate::{
    blockcfg as app_data,
    intercom::{self, BlockMsg, ClientMsg, RequestSink, TopologyMsg},
    topology::{self, Gossips, NodeId},
    utils::async_msg::MessageBox,
};
use async_trait::async_trait;
use chain_network::{
    core::server::{BlockService, FragmentService, GossipService, Node, PushStream},
    data::{
        p2p::{AuthenticatedNodeId, Peer},
        Block, BlockId, BlockIds, Fragment, FragmentIds, Gossip, HandshakeResponse, Header,
    },
    error::{Code as ErrorCode, Error},
};
use futures::{prelude::*, try_join};
use std::convert::TryFrom;
use tracing::{instrument, Span};
use tracing_futures::Instrument;

#[derive(Clone)]
pub struct NodeService {
    channels: Channels,
    global_state: GlobalStateR,
}

impl NodeService {
    pub fn new(channels: Channels, global_state: GlobalStateR) -> Self {
        NodeService {
            channels,
            global_state,
        }
    }

    async fn peer_id(&self, addr: Address) -> Result<NodeId, Error> {
        self.global_state
            .peers
            .client_id(addr)
            .await
            .ok_or_else(|| Error::new(ErrorCode::FailedPrecondition, "handshake not performed"))
    }
}

#[async_trait]
impl Node for NodeService {
    type BlockService = Self;
    type FragmentService = Self;
    type GossipService = Self;

    async fn handshake(&self, peer: Peer, nonce: &[u8]) -> Result<HandshakeResponse, Error> {
        let block0_id = BlockId::try_from(self.global_state.block0_hash.as_bytes()).unwrap();
        let keypair = &self.global_state.keypair;
        let auth = keypair.sign(nonce);
        let addr = peer.addr();
        let nonce = self.global_state.peers.generate_auth_nonce(addr).await;

        Ok(HandshakeResponse {
            block0_id,
            auth,
            nonce: nonce.into(),
        })
    }

    /// Handles client ID authentication.
    async fn client_auth(&self, peer: Peer, auth: AuthenticatedNodeId) -> Result<(), Error> {
        let addr = peer.addr();
        let id = auth.id().clone().decode()?;
        self.global_state
            .peers
            .server_complete_handshake(addr, id, |nonce| auth.verify(&nonce[..]))
            .await?;
        Ok(())
    }

    fn block_service(&self) -> Option<&Self::BlockService> {
        Some(self)
    }

    fn fragment_service(&self) -> Option<&Self::FragmentService> {
        Some(self)
    }

    fn gossip_service(&self) -> Option<&Self::GossipService> {
        Some(self)
    }
}

async fn send_message<T>(mut mbox: MessageBox<T>, msg: T) -> Result<(), Error> {
    mbox.send(msg).await.map_err(|e| {
        tracing::error!(
            reason = %e,
            "failed to enqueue message for processing"
        );
        Error::new(ErrorCode::Internal, e)
    })
}

type SubscriptionStream<S> =
    stream::Map<S, fn(<S as Stream>::Item) -> Result<<S as Stream>::Item, Error>>;

fn serve_subscription<S: Stream>(sub: Option<S>) -> Result<SubscriptionStream<S>, Error> {
    Ok(sub
        .ok_or_else(|| Error::new(ErrorCode::FailedPrecondition, "handshake not performed"))?
        .map(Ok))
}

// extracted as an external function as a workaround for
// https://github.com/dtolnay/async-trait/issues/144
async fn join_streams<T, V, E, R>(
    stream: PushStream<T>,
    sink: RequestSink<<T as Decode>::Object>,
    reply: V,
) -> Result<(), Error>
where
    T: Decode,
    E: Into<Error>,
    V: Future<Output = Result<R, E>>,
{
    try_join!(
        stream
            .and_then(|header| async { header.decode() })
            .forward(sink.sink_err_into()),
        reply.err_into::<Error>(),
    )?;
    Ok(())
}

#[async_trait]
impl BlockService for NodeService {
    type PullBlocksStream = ResponseStream<app_data::Block>;
    type PullBlocksToTipStream = ResponseStream<app_data::Block>;
    type GetBlocksStream = ResponseStream<app_data::Block>;
    type PullHeadersStream = ResponseStream<app_data::Header>;
    type GetHeadersStream = ResponseStream<app_data::Header>;
    type SubscriptionStream = SubscriptionStream<BlockEventSubscription>;

    #[instrument(level = "debug", skip(self))]
    async fn tip(&self) -> Result<Header, Error> {
        let (reply_handle, reply_future) = intercom::unary_reply();
        let mbox = self.channels.client_box.clone();
        send_message(mbox, ClientMsg::GetBlockTip(reply_handle)).await?;
        let header = reply_future.await?;
        Ok(header.encode())
    }

    #[instrument(level = "debug", skip(self))]
    async fn pull_blocks(
        &self,
        from: BlockIds,
        to: BlockId,
    ) -> Result<Self::PullBlocksStream, Error> {
        let from = from.decode()?;
        let to = to.decode()?;
        let (handle, future) = intercom::stream_reply(buffer_sizes::outbound::BLOCKS);
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::PullBlocks(from, to, handle)).await?;
        let stream = future.await?;
        Ok(convert::response_stream(stream))
    }

    #[instrument(level = "debug", skip(self))]
    async fn pull_blocks_to_tip(
        &self,
        from: BlockIds,
    ) -> Result<Self::PullBlocksToTipStream, Error> {
        let from = from.decode()?;
        let (handle, future) = intercom::stream_reply(buffer_sizes::outbound::BLOCKS);
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::PullBlocksToTip(from, handle)).await?;
        let stream = future.await?;
        Ok(convert::response_stream(stream))
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_blocks(&self, ids: BlockIds) -> Result<Self::GetBlocksStream, Error> {
        let ids = ids.decode()?;
        let (handle, future) = intercom::stream_reply(buffer_sizes::outbound::BLOCKS);
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::GetBlocks(ids, handle)).await?;
        let stream = future.await?;
        Ok(convert::response_stream(stream))
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_headers(&self, ids: BlockIds) -> Result<Self::GetHeadersStream, Error> {
        let ids = ids.decode()?;
        let (handle, future) = intercom::stream_reply(buffer_sizes::outbound::HEADERS);
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::GetHeaders(ids, handle)).await?;
        let stream = future.await?;
        Ok(convert::response_stream(stream))
    }

    #[instrument(level = "debug", skip(self))]
    async fn pull_headers(
        &self,
        from: BlockIds,
        to: BlockId,
    ) -> Result<Self::PullHeadersStream, Error> {
        let from = from.decode()?;
        let to = to.decode()?;
        let (handle, future) = intercom::stream_reply(buffer_sizes::outbound::HEADERS);
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::PullHeaders(from, to, handle)).await?;
        let stream = future.await?;
        Ok(convert::response_stream(stream))
    }

    #[instrument(level = "debug", skip(self, stream))]
    async fn push_headers(&self, stream: PushStream<Header>) -> Result<(), Error> {
        let (handle, sink, reply) = intercom::stream_request(buffer_sizes::inbound::HEADERS);
        let block_box = self.channels.block_box.clone();
        send_message(block_box, BlockMsg::ChainHeaders(handle)).await?;
        join_streams(stream, sink, reply).await
    }

    #[instrument(level = "debug", skip(self, stream))]
    async fn upload_blocks(&self, stream: PushStream<Block>) -> Result<(), Error> {
        let (handle, sink, reply) = intercom::stream_request(buffer_sizes::inbound::BLOCKS);
        let block_box = self.channels.block_box.clone();
        send_message(block_box, BlockMsg::NetworkBlocks(handle)).await?;
        join_streams(stream, sink, reply).await
    }

    #[instrument(level = "debug", skip_all, fields(addr = %subscriber, id))]
    async fn block_subscription(
        &self,
        subscriber: Peer,
        stream: PushStream<Header>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let peer_id = self.peer_id(subscriber.addr()).await?;
        Span::current().record("id", peer_id.to_string().as_str());
        self.global_state.spawn(
            subscription::process_block_announcements(
                stream,
                self.channels.block_box.clone(),
                peer_id,
                self.global_state.clone(),
            )
            .in_current_span(),
        );

        let outbound = self
            .global_state
            .peers
            .subscribe_to_block_events(&peer_id)
            .await;
        Ok(serve_subscription(outbound)?)
    }
}

#[async_trait]
impl FragmentService for NodeService {
    type GetFragmentsStream = ResponseStream<app_data::Fragment>;
    type SubscriptionStream = SubscriptionStream<FragmentSubscription>;

    async fn get_fragments(&self, _ids: FragmentIds) -> Result<Self::GetFragmentsStream, Error> {
        Err(Error::unimplemented())
    }

    #[instrument(level = "debug", skip_all, fields(direction = "in", addr = %subscriber, id))]
    async fn fragment_subscription(
        &self,
        subscriber: Peer,
        stream: PushStream<Fragment>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let peer_id = self.peer_id(subscriber.addr()).await?;
        Span::current().record("id", peer_id.to_string().as_str());
        self.global_state.spawn(
            subscription::process_fragments(
                stream,
                self.channels.transaction_box.clone(),
                peer_id,
                self.global_state.clone(),
            )
            .in_current_span(),
        );

        let outbound = self
            .global_state
            .peers
            .subscribe_to_fragments(&peer_id)
            .await;
        Ok(serve_subscription(outbound)?)
    }
}

#[async_trait]
impl GossipService for NodeService {
    type SubscriptionStream = SubscriptionStream<GossipSubscription>;

    #[instrument(level = "debug", skip_all, fields(direction = "in", addr = %subscriber, id))]
    async fn gossip_subscription(
        &self,
        subscriber: Peer,
        stream: PushStream<Gossip>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let peer_id = self.peer_id(subscriber.addr()).await?;
        Span::current().record("id", peer_id.to_string().as_str());
        self.global_state.spawn(
            subscription::process_gossip(
                stream,
                self.channels.topology_box.clone(),
                peer_id,
                self.global_state.clone(),
            )
            .in_current_span(),
        );

        let outbound = self.global_state.peers.subscribe_to_gossip(&peer_id).await;
        Ok(serve_subscription(outbound)?)
    }

    #[instrument(level = "debug", skip(self))]
    async fn peers(&self, limit: u32) -> Result<Gossip, Error> {
        let (reply_handle, reply_future) = intercom::unary_reply();
        let mbox = self.channels.topology_box.clone();
        send_message(
            mbox,
            TopologyMsg::View(poldercast::layer::Selection::Any, reply_handle),
        )
        .await?;
        let res = reply_future.await?;
        let gossip = Gossips::from(
            std::iter::once(res.self_node)
                .chain(res.peers.into_iter())
                .take(limit as usize)
                .map(topology::Gossip::from)
                .collect::<Vec<_>>(),
        )
        .encode();
        Ok(gossip)
    }
}
