use super::{p2p_topology as p2p, propagate::Subscription, subscription, Channels, GlobalStateR};

use crate::blockcfg::{Block, BlockDate, Header, HeaderHash, Message, MessageId};
use crate::intercom::{self, stream_reply, unary_reply, ClientMsg, ReplyFuture, ReplyStream};

use network_core::{
    error as core_error,
    gossip::{Gossip, Node as _},
    server::{
        block::BlockService,
        content::{ContentService, ProposeTransactionsResponse},
        gossip::GossipService,
        Node, P2pService,
    },
};

use futures::future::{self, FutureResult};
use futures::prelude::*;

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
}

impl Node for NodeService {
    type BlockService = Self;
    type ContentService = Self;
    type GossipService = Self;

    fn block_service(&mut self) -> Option<&mut Self::BlockService> {
        Some(self)
    }

    fn content_service(&mut self) -> Option<&mut Self::ContentService> {
        // Not implemented yet
        None
    }

    fn gossip_service(&mut self) -> Option<&mut Self::GossipService> {
        Some(self)
    }
}

impl From<intercom::Error> for core_error::Error {
    fn from(err: intercom::Error) -> Self {
        core_error::Error::new(err.code(), err)
    }
}

impl P2pService for NodeService {
    type NodeId = p2p::NodeId;

    fn node_id(&self) -> p2p::NodeId {
        self.global_state.node.id()
    }
}

impl BlockService for NodeService {
    type BlockId = HeaderHash;
    type BlockDate = BlockDate;
    type Block = Block;
    type TipFuture = ReplyFuture<Header, core_error::Error>;
    type Header = Header;
    type PullBlocksStream = ReplyStream<Block, core_error::Error>;
    type PullBlocksFuture = FutureResult<Self::PullBlocksStream, core_error::Error>;
    type GetBlocksStream = ReplyStream<Block, core_error::Error>;
    type GetBlocksFuture = FutureResult<Self::GetBlocksStream, core_error::Error>;
    type PullHeadersStream = ReplyStream<Header, core_error::Error>;
    type PullHeadersFuture = FutureResult<Self::PullHeadersStream, core_error::Error>;
    type GetHeadersStream = ReplyStream<Header, core_error::Error>;
    type GetHeadersFuture = FutureResult<Self::GetHeadersStream, core_error::Error>;
    type BlockSubscription = Subscription<Header>;
    type BlockSubscriptionFuture = FutureResult<Self::BlockSubscription, core_error::Error>;

    fn tip(&mut self) -> Self::TipFuture {
        let (handle, future) = unary_reply();
        self.channels
            .client_box
            .send_to(ClientMsg::GetBlockTip(handle));
        future
    }

    fn pull_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::PullBlocksFuture {
        let (handle, stream) = stream_reply();
        self.channels
            .client_box
            .send_to(ClientMsg::PullBlocksToTip(from.into(), handle));
        future::ok(stream)
    }

    fn get_blocks(&mut self, ids: &[Self::BlockId]) -> Self::GetBlocksFuture {
        let (handle, stream) = stream_reply();
        self.channels
            .client_box
            .send_to(ClientMsg::GetBlocks(ids.into(), handle));
        future::ok(stream)
    }

    fn get_headers(&mut self, ids: &[Self::BlockId]) -> Self::GetHeadersFuture {
        let (handle, stream) = stream_reply();
        self.channels
            .client_box
            .send_to(ClientMsg::GetHeaders(ids.into(), handle));
        future::ok(stream)
    }

    fn pull_blocks_to(
        &mut self,
        _from: &[Self::BlockId],
        _to: &Self::BlockId,
    ) -> Self::PullBlocksFuture {
        unimplemented!()
    }

    fn pull_headers_to(
        &mut self,
        _from: &[Self::BlockId],
        _to: &Self::BlockId,
    ) -> Self::PullHeadersFuture {
        unimplemented!()
    }

    fn pull_headers_to_tip(&mut self, _from: &[Self::BlockId]) -> Self::PullHeadersFuture {
        unimplemented!()
    }

    fn block_subscription<In>(
        &mut self,
        subscriber: Self::NodeId,
        inbound: In,
    ) -> Self::BlockSubscriptionFuture
    where
        In: Stream<Item = Self::Header, Error = core_error::Error> + Send + 'static,
    {
        tokio::spawn(subscription::process_blocks(
            inbound,
            self.channels.block_box.clone(),
        ));

        let subscription = self
            .global_state
            .propagation_peers
            .subscribe_to_blocks(subscriber);
        future::ok(subscription)
    }
}

impl ContentService for NodeService {
    type Message = Message;
    type MessageId = MessageId;
    type ProposeTransactionsFuture =
        ReplyFuture<ProposeTransactionsResponse<MessageId>, core_error::Error>;
    type GetMessagesStream = ReplyStream<Self::Message, core_error::Error>;
    type GetMessagesFuture = ReplyFuture<Self::GetMessagesStream, core_error::Error>;
    type MessageSubscription = Subscription<Message>;
    type MessageSubscriptionFuture = FutureResult<Self::MessageSubscription, core_error::Error>;

    fn propose_transactions(
        &mut self,
        _ids: &[Self::MessageId],
    ) -> Self::ProposeTransactionsFuture {
        unimplemented!()
    }

    fn get_messages(&mut self, _ids: &[Self::MessageId]) -> Self::GetMessagesFuture {
        unimplemented!()
    }

    fn message_subscription<S>(
        &mut self,
        subscriber: Self::NodeId,
        _inbound: S,
    ) -> Self::MessageSubscriptionFuture
    where
        S: Stream<Item = Self::Message, Error = core_error::Error>,
    {
        unimplemented!()
    }
}

impl GossipService for NodeService {
    type Node = p2p::Node;
    type GossipSubscription = Subscription<Gossip<p2p::Node>>;
    type GossipSubscriptionFuture = FutureResult<Self::GossipSubscription, core_error::Error>;

    fn gossip_subscription<In>(
        &mut self,
        subscriber: Self::NodeId,
        inbound: In,
    ) -> Self::GossipSubscriptionFuture
    where
        In: Stream<Item = Gossip<Self::Node>, Error = core_error::Error> + Send + 'static,
    {
        tokio::spawn(subscription::process_gossip(
            inbound,
            self.global_state.clone(),
        ));

        let subscription = self
            .global_state
            .propagation_peers
            .subscribe_to_gossip(subscriber);
        future::ok(subscription)
    }
}
