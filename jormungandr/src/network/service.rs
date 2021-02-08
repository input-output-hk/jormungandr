use super::{
    buffer_sizes,
    convert::{self, Decode, Encode, ResponseStream},
    p2p::comm::{BlockEventSubscription, FragmentSubscription, GossipSubscription},
    p2p::Address,
    subscription, Channels, GlobalStateR,
};
use crate::blockcfg as app_data;
use crate::intercom::{self, BlockMsg, ClientMsg};
use crate::utils::async_msg::MessageBox;
use chain_network::core::server::{BlockService, FragmentService, GossipService, Node, PushStream};
use chain_network::data::p2p::{AuthenticatedNodeId, Peer, Peers};
use chain_network::data::{
    Block, BlockId, BlockIds, Fragment, FragmentIds, Gossip, HandshakeResponse, Header,
};
use chain_network::error::{Code as ErrorCode, Error};

use async_trait::async_trait;
use futures::prelude::*;
use futures::try_join;
use tracing::{span, Level, Span};
use tracing_futures::Instrument;

use std::convert::TryFrom;

#[derive(Clone)]
pub struct NodeService {
    channels: Channels,
    global_state: GlobalStateR,
    span: Span,
}

impl NodeService {
    pub fn new(channels: Channels, global_state: GlobalStateR) -> Self {
        let span = span!(parent: global_state.span(), Level::TRACE, "sub_task", kind = "server");
        NodeService {
            channels,
            span,
            global_state,
        }
    }

    pub fn span(&self) -> &Span {
        &self.span
    }
}

impl NodeService {
    fn subscription_span(&self, subscriber: Peer, stream_name: &'static str) -> Span {
        span!(parent: self.span(), Level::TRACE, "NodeService", peer = %subscriber.to_string(), stream = %stream_name)
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
        let addr = Address::tcp(peer.addr());
        let nonce = self.global_state.peers.generate_auth_nonce(addr).await;

        Ok(HandshakeResponse {
            block0_id,
            auth,
            nonce: nonce.into(),
        })
    }

    /// Handles client ID authentication.
    async fn client_auth(&self, peer: Peer, auth: AuthenticatedNodeId) -> Result<(), Error> {
        let addr = Address::tcp(peer.addr());
        let nonce = self.global_state.peers.get_auth_nonce(addr.clone()).await;
        let nonce = nonce.ok_or_else(|| {
            Error::new(
                ErrorCode::FailedPrecondition,
                "nonce is missing, perform Handshake first",
            )
        })?;
        auth.verify(&nonce[..])?;
        self.global_state.peers.set_node_id(addr, auth.into()).await;
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

fn serve_subscription<S: Stream>(sub: S) -> SubscriptionStream<S> {
    sub.map(Ok)
}

#[async_trait]
impl BlockService for NodeService {
    type PullBlocksStream = ResponseStream<app_data::Block>;
    type PullBlocksToTipStream = ResponseStream<app_data::Block>;
    type GetBlocksStream = ResponseStream<app_data::Block>;
    type PullHeadersStream = ResponseStream<app_data::Header>;
    type GetHeadersStream = ResponseStream<app_data::Header>;
    type SubscriptionStream = SubscriptionStream<BlockEventSubscription>;

    async fn tip(&self) -> Result<Header, Error> {
        let span = span!(Level::TRACE, "request", kind = "Tip");
        let (reply_handle, reply_future) = intercom::unary_reply();
        let reply_future = reply_future.instrument(span.clone());
        let mbox = self.channels.client_box.clone();
        send_message(mbox, ClientMsg::GetBlockTip(reply_handle))
            .instrument(span)
            .await?;
        let header = reply_future.await?;
        Ok(header.encode())
    }

    async fn pull_blocks(
        &self,
        from: BlockIds,
        to: BlockId,
    ) -> Result<Self::PullBlocksStream, Error> {
        let from = from.decode()?;
        let to = to.decode()?;
        let span = span!(Level::TRACE, "request", kind = "PullBlocks");
        let (handle, future) = intercom::stream_reply(buffer_sizes::outbound::BLOCKS);
        let future = future.instrument(span.clone());
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::PullBlocks(from, to, handle))
            .instrument(span)
            .await?;
        let stream = future.await?;
        Ok(convert::response_stream(stream))
    }

    async fn pull_blocks_to_tip(
        &self,
        from: BlockIds,
    ) -> Result<Self::PullBlocksToTipStream, Error> {
        let from = from.decode()?;
        let span = span!(Level::TRACE, "request", kind = "PullBlocksToTip");
        let (handle, future) = intercom::stream_reply(buffer_sizes::outbound::BLOCKS);
        let future = future.instrument(span.clone());
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::PullBlocksToTip(from, handle))
            .instrument(span)
            .await?;
        let stream = future.await?;
        Ok(convert::response_stream(stream))
    }

    async fn get_blocks(&self, ids: BlockIds) -> Result<Self::GetBlocksStream, Error> {
        let ids = ids.decode()?;
        let span = span!(Level::TRACE, "request", kind = "GetBlocks");
        let (handle, future) = intercom::stream_reply(buffer_sizes::outbound::BLOCKS);
        let future = future.instrument(span.clone());
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::GetBlocks(ids, handle)).await?;
        let stream = future.await?;
        Ok(convert::response_stream(stream))
    }

    async fn get_headers(&self, ids: BlockIds) -> Result<Self::GetHeadersStream, Error> {
        let ids = ids.decode()?;
        let span = span!(Level::TRACE, "request", kind = "GetHeaders");
        let (handle, future) = intercom::stream_reply(buffer_sizes::outbound::HEADERS);
        let future = future.instrument(span.clone());
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::GetHeaders(ids, handle))
            .instrument(span)
            .await?;
        let stream = future.await?;
        Ok(convert::response_stream(stream))
    }

    async fn pull_headers(
        &self,
        from: BlockIds,
        to: BlockId,
    ) -> Result<Self::PullHeadersStream, Error> {
        let from = from.decode()?;
        let to = to.decode()?;
        let span = span!(Level::TRACE, "request", kind = "PullHeaders");
        let (handle, future) = intercom::stream_reply(buffer_sizes::outbound::HEADERS);
        let future = future.instrument(span.clone());
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::GetHeadersRange(from, to, handle))
            .instrument(span)
            .await?;
        let stream = future.await?;
        Ok(convert::response_stream(stream))
    }

    async fn push_headers(&self, stream: PushStream<Header>) -> Result<(), Error> {
        let span = span!(Level::TRACE, "request", kind = "PushHeaders");
        let (handle, sink, reply) = intercom::stream_request(buffer_sizes::inbound::HEADERS);
        let reply = reply.instrument(span.clone());
        let block_box = self.channels.block_box.clone();
        send_message(block_box, BlockMsg::ChainHeaders(handle))
            .instrument(span)
            .await?;
        try_join!(
            stream
                .and_then(|header| async { header.decode() })
                .forward(sink.sink_err_into()),
            reply.err_into(),
        )?;
        Ok(())
    }

    async fn upload_blocks(&self, stream: PushStream<Block>) -> Result<(), Error> {
        let span = span!(Level::TRACE, "request", kind = "UploadBlocks");
        let (handle, sink, reply) = intercom::stream_request(buffer_sizes::inbound::BLOCKS);
        let reply = reply.instrument(span.clone());
        let block_box = self.channels.block_box.clone();
        send_message(block_box, BlockMsg::NetworkBlocks(handle))
            .instrument(span)
            .await?;
        try_join!(
            stream
                .and_then(|block| async { block.decode() })
                .forward(sink.sink_err_into()),
            reply.err_into(),
        )?;
        Ok(())
    }

    async fn block_subscription(
        &self,
        subscriber: Peer,
        stream: PushStream<Header>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let addr = subscriber.addr();
        let span = self.subscription_span(subscriber, "block_events");
        let subscriber = Address::tcp(addr);

        self.global_state.spawn(
            subscription::process_block_announcements(
                stream,
                self.channels.block_box.clone(),
                subscriber.clone(),
                self.global_state.clone(),
                span.clone(),
            )
            .instrument(span),
        );

        let outbound = self
            .global_state
            .peers
            .subscribe_to_block_events(subscriber)
            .await;
        Ok(serve_subscription(outbound))
    }
}

#[async_trait]
impl FragmentService for NodeService {
    type GetFragmentsStream = ResponseStream<app_data::Fragment>;
    type SubscriptionStream = SubscriptionStream<FragmentSubscription>;

    async fn get_fragments(&self, _ids: FragmentIds) -> Result<Self::GetFragmentsStream, Error> {
        Err(Error::unimplemented())
    }

    async fn fragment_subscription(
        &self,
        subscriber: Peer,
        stream: PushStream<Fragment>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let addr = subscriber.addr();
        let parent_span = self.subscription_span(subscriber, "fragments");
        let subscriber = Address::tcp(addr);
        let span = span!(
            parent: parent_span,
            Level::TRACE,
            "fragment_subscription",
            direction = "in"
        );
        self.global_state.spawn(
            subscription::process_fragments(
                stream,
                self.channels.transaction_box.clone(),
                subscriber.clone(),
                self.global_state.clone(),
                span.clone(),
            )
            .instrument(span),
        );

        let outbound = self
            .global_state
            .peers
            .subscribe_to_fragments(subscriber)
            .await;
        Ok(serve_subscription(outbound))
    }
}

#[async_trait]
impl GossipService for NodeService {
    type SubscriptionStream = SubscriptionStream<GossipSubscription>;

    async fn gossip_subscription(
        &self,
        subscriber: Peer,
        stream: PushStream<Gossip>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let addr = subscriber.addr();
        let parent_span = self.subscription_span(subscriber, "gossip");
        let subscriber = Address::tcp(addr);
        let span = span!(
            parent: parent_span,
            Level::TRACE,
            "fragment_subscription",
            direction = "in"
        );

        self.global_state
            .spawn(subscription::process_gossip(
                stream,
                subscriber.clone(),
                self.global_state.clone(),
                span.clone(),
            ))
            .instrument(span);

        let outbound = self
            .global_state
            .peers
            .subscribe_to_gossip(subscriber)
            .await;
        Ok(serve_subscription(outbound))
    }

    async fn peers(&self, limit: u32) -> Result<Peers, Error> {
        let topology = &self.global_state.topology;
        let view = topology.view(poldercast::Selection::Any).await;
        let mut peers = Vec::new();
        for n in view.peers.into_iter() {
            if let Some(addr) = n.to_socket_addr() {
                peers.push(addr.into());
                if peers.len() >= limit as usize {
                    break;
                }
            }
        }
        if peers.is_empty() {
            // No peers yet, put self as the peer to bootstrap from
            if let Some(addr) = view.self_node.address().and_then(|x| x.to_socket_addr()) {
                peers.push(addr.into());
            }
        }
        Ok(peers.into_boxed_slice())
    }
}
