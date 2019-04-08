use super::{p2p_topology as p2p, propagate::Subscription, Channels, GlobalState};

use crate::blockcfg::{Block, BlockDate, Header, HeaderHash, Message, MessageId};
use crate::intercom::{
    self, stream_reply, unary_reply, BlockMsg, ClientMsg, ReplyFuture, ReplyStream,
};

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

use futures::prelude::*;
use futures::{
    future::{self, FutureResult},
    stream,
};

use std::sync::Arc;

#[derive(Clone)]
pub struct NodeService {
    channels: Channels,
    global_state: Arc<GlobalState>,
}

impl NodeService {
    pub fn new(channels: Channels, global_state: Arc<GlobalState>) -> Self {
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
    type BlockSubscription = stream::Empty<Header, core_error::Error>;
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
        let mut block_box = self.channels.block_box.clone();
        tokio::spawn(
            inbound
                .for_each(move |header| {
                    block_box.send(BlockMsg::AnnouncedBlock(header));
                    future::ok(())
                })
                .map_err(|err| {
                    error!("Block subscription failed: {:?}", err);
                }),
        );

        // FIXME: we can't have per-connection state associated with
        // NodeService in the current tower-h2 design. Need to come up
        // with a way to identify the peer making the subscription, so that
        // we can use this stream for p2p propagation.
        // See https://github.com/tower-rs/tower-h2/issues/64
        future::ok(stream::empty())
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
    type GossipSubscription = stream::Empty<Gossip<p2p::Node>, core_error::Error>;
    type GossipSubscriptionFuture = FutureResult<Self::GossipSubscription, core_error::Error>;

    fn gossip_subscription<In>(
        &mut self,
        subscriber: Self::NodeId,
        inbound: In,
    ) -> Self::GossipSubscriptionFuture
    where
        In: Stream<Item = Gossip<Self::Node>, Error = core_error::Error> + Send + 'static,
    {
        let global_state = self.global_state.clone();
        tokio::spawn(
            inbound
                .for_each(move |gossip| {
                    global_state.topology.update(gossip.into_nodes());
                    Ok(())
                })
                .map_err(|err| {
                    error!("gossip subscription inbound stream error: {:?}", err);
                }),
        );

        // TODO: send periodic updates to nodes
        // See the BlockService::block_subscription impl for why this is
        // currently not implemented.
        future::ok(stream::empty())
    }
}
