use super::{
    block_pull::BlockPull,
    p2p::comm::{BlockEventSubscription, Subscription},
    p2p::topology,
    subscription, Channels, GlobalStateR,
};
use crate::blockcfg::{Block, BlockDate, Header, HeaderHash, Message, MessageId};
use crate::intercom::{stream_reply, unary_reply, BlockMsg, ClientMsg, ReplyFuture, ReplyStream};
use futures::future::{self, FutureResult};
use futures::prelude::*;
use network_core::{
    error as core_error,
    gossip::{Gossip, Node as _},
    server::{
        block::BlockService, content::ContentService, gossip::GossipService, Node, P2pService,
    },
};
use slog::Logger;

#[derive(Clone)]
pub struct NodeService {
    channels: Channels,
    global_state: GlobalStateR,
    logger: Logger,
}

impl NodeService {
    pub fn new(channels: Channels, global_state: GlobalStateR) -> Self {
        NodeService {
            channels,
            logger: global_state.logger().new(o!(::log::KEY_TASK => "server")),
            global_state,
        }
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
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

impl P2pService for NodeService {
    type NodeId = topology::NodeId;

    fn node_id(&self) -> topology::NodeId {
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
    type PullBlocksToTipFuture = FutureResult<Self::PullBlocksStream, core_error::Error>;
    type GetBlocksStream = ReplyStream<Block, core_error::Error>;
    type GetBlocksFuture = FutureResult<Self::GetBlocksStream, core_error::Error>;
    type PullHeadersStream = ReplyStream<Header, core_error::Error>;
    type PullHeadersFuture = FutureResult<Self::PullHeadersStream, core_error::Error>;
    type GetHeadersStream = ReplyStream<Header, core_error::Error>;
    type GetHeadersFuture = FutureResult<Self::GetHeadersStream, core_error::Error>;
    type PushHeadersFuture = FutureResult<(), core_error::Error>;
    type OnUploadedBlockFuture = FutureResult<(), core_error::Error>;
    type BlockSubscription = BlockEventSubscription;
    type BlockSubscriptionFuture = FutureResult<Self::BlockSubscription, core_error::Error>;

    fn tip(&mut self) -> Self::TipFuture {
        let (handle, future) = unary_reply(self.logger().clone());
        self.channels
            .client_box
            .send_to(ClientMsg::GetBlockTip(handle));
        future
    }

    fn pull_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::PullBlocksFuture {
        let (handle, stream) = stream_reply(self.logger().clone());
        self.channels
            .client_box
            .send_to(ClientMsg::PullBlocksToTip(from.into(), handle));
        future::ok(stream)
    }

    fn get_blocks(&mut self, ids: &[Self::BlockId]) -> Self::GetBlocksFuture {
        let (handle, stream) = stream_reply(self.logger().clone());
        self.channels
            .client_box
            .send_to(ClientMsg::GetBlocks(ids.into(), handle));
        future::ok(stream)
    }

    fn get_headers(&mut self, ids: &[Self::BlockId]) -> Self::GetHeadersFuture {
        let (handle, stream) = stream_reply(self.logger().clone());
        self.channels
            .client_box
            .send_to(ClientMsg::GetHeaders(ids.into(), handle));
        future::ok(stream)
    }

    fn pull_blocks(
        &mut self,
        _from: &[Self::BlockId],
        _to: &Self::BlockId,
    ) -> Self::PullBlocksFuture {
        unimplemented!()
    }

    fn pull_headers(
        &mut self,
        from: &[Self::BlockId],
        to: &Self::BlockId,
    ) -> Self::PullHeadersFuture {
        let (handle, stream) = stream_reply(self.logger().clone());
        self.channels
            .client_box
            .send_to(ClientMsg::GetHeadersRange(from.into(), *to, handle));
        future::ok(stream)
    }

    fn pull_headers_to_tip(&mut self, _from: &[Self::BlockId]) -> Self::PullHeadersFuture {
        unimplemented!()
    }

    fn push_headers<In>(&mut self, headers: In) -> Self::PushHeadersFuture
    where
        In: Stream<Item = Self::Header, Error = core_error::Error> + Send + 'static,
    {
        //BlockPull::new(headers, self.channels.block_box.clone(), self.logger.clone())
        unimplemented!()
    }

    fn on_uploaded_block(&mut self, block: Block) -> Self::OnUploadedBlockFuture {
        self.channels
            .block_box
            .try_send(BlockMsg::NetworkBlock(block))
            .unwrap();
        future::ok(())
    }

    fn block_subscription<In>(
        &mut self,
        subscriber: Self::NodeId,
        inbound: In,
    ) -> Self::BlockSubscriptionFuture
    where
        In: Stream<Item = Self::Header, Error = core_error::Error> + Send + 'static,
    {
        subscription::process_block_announcements(
            inbound,
            subscriber,
            self.channels.block_box.clone(),
            self.logger().clone(),
        );

        let subscription = self
            .global_state
            .peers
            .subscribe_to_block_events(subscriber);
        future::ok(subscription)
    }
}

impl ContentService for NodeService {
    type Message = Message;
    type MessageId = MessageId;
    type GetMessagesStream = ReplyStream<Self::Message, core_error::Error>;
    type GetMessagesFuture = ReplyFuture<Self::GetMessagesStream, core_error::Error>;
    type MessageSubscription = Subscription<Message>;
    type MessageSubscriptionFuture = FutureResult<Self::MessageSubscription, core_error::Error>;

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
    type Node = topology::Node;
    type GossipSubscription = Subscription<Gossip<topology::Node>>;
    type GossipSubscriptionFuture = FutureResult<Self::GossipSubscription, core_error::Error>;

    fn gossip_subscription<In>(
        &mut self,
        subscriber: Self::NodeId,
        inbound: In,
    ) -> Self::GossipSubscriptionFuture
    where
        In: Stream<Item = Gossip<Self::Node>, Error = core_error::Error> + Send + 'static,
    {
        subscription::process_gossip(inbound, self.global_state.clone(), self.logger().clone());

        let subscription = self.global_state.peers.subscribe_to_gossip(subscriber);
        future::ok(subscription)
    }
}
