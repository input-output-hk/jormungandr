use super::{
    buffer_sizes,
    inbound::InboundProcessing,
    p2p::comm::{BlockEventSubscription, OutboundSubscription},
    p2p::topology,
    subscription::{BlockAnnouncementProcessor, FragmentProcessor, GossipProcessor, Subscription},
    Channels, GlobalStateR,
};
use crate::blockcfg::{Block, BlockDate, Fragment, FragmentId, Header, HeaderHash};
use crate::intercom::{self, BlockMsg, ClientMsg, ReplyFuture, ReplyStream, RequestSink};
use futures::future::{self, FutureResult};
use futures::prelude::*;
use network_core::error as core_error;
use network_core::gossip::{Gossip, Node as _};
use network_core::server::{BlockService, FragmentService, GossipService, Node, P2pService};
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
            logger: global_state
                .logger()
                .new(o!(::log::KEY_SUB_TASK => "server")),
            global_state,
        }
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }
}

impl NodeService
where
    Self: P2pService,
{
    fn subscription_logger(&self, subscriber: <Self as P2pService>::NodeId) -> Logger {
        self.logger.new(o!("node_id" => subscriber.to_string()))
    }
}

impl Node for NodeService {
    type BlockService = Self;
    type FragmentService = Self;
    type GossipService = Self;

    fn block_service(&mut self) -> Option<&mut Self::BlockService> {
        Some(self)
    }

    fn fragment_service(&mut self) -> Option<&mut Self::FragmentService> {
        Some(self)
    }

    fn gossip_service(&mut self) -> Option<&mut Self::GossipService> {
        Some(self)
    }
}

impl P2pService for NodeService {
    type NodeId = topology::NodeId;

    fn node_id(&self) -> topology::NodeId {
        self.global_state.topology.node().id()
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
    type PushHeadersSink = RequestSink<Header, (), core_error::Error>;
    type UploadBlocksSink = InboundProcessing<Block, BlockMsg>;
    type BlockSubscription = Subscription<BlockAnnouncementProcessor, BlockEventSubscription>;
    type BlockSubscriptionFuture = FutureResult<Self::BlockSubscription, core_error::Error>;

    fn block0(&mut self) -> HeaderHash {
        self.global_state.block0_hash
    }

    fn tip(&mut self) -> Self::TipFuture {
        let (handle, future) = intercom::unary_reply(self.logger().clone());
        self.channels
            .client_box
            .send_to(ClientMsg::GetBlockTip(handle));
        future
    }

    fn pull_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::PullBlocksFuture {
        let (handle, stream) = intercom::stream_reply(self.logger().clone());
        self.channels
            .client_box
            .send_to(ClientMsg::PullBlocksToTip(from.into(), handle));
        future::ok(stream)
    }

    fn get_blocks(&mut self, ids: &[Self::BlockId]) -> Self::GetBlocksFuture {
        let (handle, stream) = intercom::stream_reply(self.logger().clone());
        self.channels
            .client_box
            .send_to(ClientMsg::GetBlocks(ids.into(), handle));
        future::ok(stream)
    }

    fn get_headers(&mut self, ids: &[Self::BlockId]) -> Self::GetHeadersFuture {
        let (handle, stream) = intercom::stream_reply(self.logger().clone());
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
        let (handle, stream) = intercom::stream_reply(self.logger().clone());
        self.channels
            .client_box
            .send_to(ClientMsg::GetHeadersRange(from.into(), *to, handle));
        future::ok(stream)
    }

    fn pull_headers_to_tip(&mut self, _from: &[Self::BlockId]) -> Self::PullHeadersFuture {
        unimplemented!()
    }

    fn push_headers(&mut self) -> Self::PushHeadersSink {
        let logger = self.logger.new(o!("request" => "PushHeaders"));
        let (handle, sink) = intercom::stream_request(buffer_sizes::CHAIN_PULL, logger.clone());
        let block_box = self.channels.block_box.clone();
        // TODO: make sure that a limit on the number of requests in flight
        // per service connection prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state.spawn(
            block_box
                .send(BlockMsg::ChainHeaders(handle))
                .map_err(move |e| {
                    error!(
                        logger,
                        "failed to enqueue request for processing";
                        "reason" => %e,
                    );
                })
                .map(|_mbox| ()),
        );
        sink
    }

    fn upload_blocks(&mut self) -> Self::UploadBlocksSink {
        InboundProcessing::with_unary(
            self.channels.block_box.clone(),
            self.logger.clone(),
            |block, handle| BlockMsg::NetworkBlock(block, handle),
        )
    }

    fn block_subscription(&mut self, subscriber: Self::NodeId) -> Self::BlockSubscriptionFuture {
        let logger = self
            .subscription_logger(subscriber)
            .new(o!("stream" => "block_events"));

        let sink = BlockAnnouncementProcessor::new(
            self.channels.block_box.clone(),
            subscriber,
            self.global_state.clone(),
            logger.new(o!("direction" => "in")),
        );

        let outbound = self.global_state.peers.serve_block_events(subscriber);

        let subscription = Subscription::new(sink, outbound, logger);
        future::ok(subscription)
    }
}

impl FragmentService for NodeService {
    type Fragment = Fragment;
    type FragmentId = FragmentId;
    type GetFragmentsStream = ReplyStream<Self::Fragment, core_error::Error>;
    type GetFragmentsFuture = ReplyFuture<Self::GetFragmentsStream, core_error::Error>;
    type FragmentSubscription = Subscription<FragmentProcessor, OutboundSubscription<Fragment>>;
    type FragmentSubscriptionFuture = FutureResult<Self::FragmentSubscription, core_error::Error>;

    fn get_fragments(&mut self, _ids: &[Self::FragmentId]) -> Self::GetFragmentsFuture {
        unimplemented!()
    }

    fn fragment_subscription(
        &mut self,
        subscriber: Self::NodeId,
    ) -> Self::FragmentSubscriptionFuture {
        let logger = self
            .subscription_logger(subscriber)
            .new(o!("stream" => "fragments"));

        let sink = FragmentProcessor::new(
            self.channels.transaction_box.clone(),
            subscriber,
            self.global_state.clone(),
            logger.new(o!("direction" => "in")),
        );

        let outbound = self.global_state.peers.serve_fragments(subscriber);

        let subscription = Subscription::new(sink, outbound, logger);
        future::ok(subscription)
    }
}

impl GossipService for NodeService {
    type Node = topology::NodeData;
    type GossipSubscription =
        Subscription<GossipProcessor, OutboundSubscription<Gossip<topology::NodeData>>>;
    type GossipSubscriptionFuture = FutureResult<Self::GossipSubscription, core_error::Error>;

    fn gossip_subscription(&mut self, subscriber: Self::NodeId) -> Self::GossipSubscriptionFuture {
        let logger = self
            .subscription_logger(subscriber)
            .new(o!("stream" => "gossip"));

        let sink = GossipProcessor::new(
            subscriber,
            self.global_state.clone(),
            logger.new(o!("direction" => "in")),
        );

        let outbound = self.global_state.peers.serve_gossip(subscriber);

        let subscription = Subscription::new(sink, outbound, logger);
        future::ok(subscription)
    }
}
