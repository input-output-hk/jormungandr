use super::{
    buffer_sizes,
    p2p::comm::{BlockEventSubscription, OutboundSubscription},
    p2p::{Gossip as NodeData, Id},
    subscription::{
        self, BlockAnnouncementProcessor, FragmentProcessor, GossipProcessor, Subscription,
    },
    Channels, GlobalStateR,
};
use crate::blockcfg::{Block, BlockDate, Fragment, FragmentId, Header, HeaderHash};
use crate::intercom::{self, BlockMsg, ClientMsg, ReplyStream};
use chain_network::core::server::{BlockService, FragmentService, GossipService, Node, PushStream};
use chain_network::data as net_data;
use chain_network::data::gossip::{Gossip, Peers};
use chain_network::error::{self as net_error, Error};

use async_trait::async_trait;
use futures03::prelude::*;
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
                .new(o!(crate::log::KEY_SUB_TASK => "server")),
            global_state,
        }
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }
}

impl NodeService {
    fn subscription_logger(&self, subscriber: Id) -> Logger {
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

#[async_trait]
impl BlockService for NodeService {
    type PullBlocksToTipStream = ReplyStream<Block, Error>;
    type GetBlocksStream = ReplyStream<Block, Error>;
    type PullHeadersStream = ReplyStream<Header, Error>;
    type GetHeadersStream = ReplyStream<Header, Error>;
    type SubscriptionStream = ReplyStream<net_data::BlockEvent, Error>;

    fn block0(&mut self) -> net_data::BlockId {
        net_data::BlockId::try_from(self.global_state.block0_hash.as_bytes()).unwrap()
    }

    async fn tip(&self) -> Result<Header, Error> {
        intercom::unary_future(
            self.channels.client_box.clone(),
            self.logger().new(o!("request" => "Tip")),
            ClientMsg::GetBlockTip,
        )
    }

    async fn pull_blocks_to_tip(
        &self,
        from: net_data::BlockIds,
    ) -> Result<Self::PullBlocksToTipStream, Error> {
        let logger = self.logger().new(o!("request" => "PullBlocksToTip"));
        let (handle, stream) =
            intercom::stream_reply(buffer_sizes::outbound::BLOCKS, logger.clone());
        let client_box = self.channels.client_box.clone();
        // TODO: make sure that a limit on the number of requests in flight
        // per service connection prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state.spawn(
            client_box.into_send_task(ClientMsg::PullBlocksToTip(from.into(), handle), logger),
        );
        future::ok(stream)
    }

    async fn get_blocks(&self, ids: net_data::BlockIds) -> Result<Self::GetBlocksStream, Error> {
        let logger = self.logger().new(o!("request" => "GetBlocks"));
        let (handle, stream) =
            intercom::stream_reply(buffer_sizes::outbound::BLOCKS, logger.clone());
        let client_box = self.channels.client_box.clone();
        // TODO: make sure that a limit on the number of requests in flight
        // per service connection prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state
            .spawn(client_box.into_send_task(ClientMsg::GetBlocks(ids.into(), handle), logger));
        future::ok(stream)
    }

    async fn get_headers(&self, ids: net_data::BlockIds) -> Result<Self::GetHeadersStream, Error> {
        let logger = self.logger().new(o!("request" => "GetHeaders"));
        let (handle, stream) =
            intercom::stream_reply(buffer_sizes::outbound::HEADERS, logger.clone());
        let client_box = self.channels.client_box.clone();
        // TODO: make sure that a limit on the number of requests in flight
        // per service connection prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state
            .spawn(client_box.into_send_task(ClientMsg::GetHeaders(ids.into(), handle), logger));
        future::ok(stream)
    }

    async fn pull_headers(
        &self,
        from: net_data::BlockIds,
        to: net_data::BlockId,
    ) -> Result<Self::PullHeadersStream, Error> {
        let logger = self.logger().new(o!("request" => "PullHeaders"));
        let (handle, stream) =
            intercom::stream_reply(buffer_sizes::outbound::HEADERS, logger.clone());
        let client_box = self.channels.client_box.clone();
        // TODO: make sure that a limit on the number of requests in flight
        // per service connection prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state.spawn(
            client_box.into_send_task(ClientMsg::GetHeadersRange(from.into(), *to, handle), logger),
        );
        future::ok(stream)
    }

    async fn push_headers(&self, stream: PushStream<Header>) -> Result<(), Error> {
        let logger = self.logger.new(o!("request" => "PushHeaders"));
        let (handle, sink) =
            intercom::stream_request(buffer_sizes::inbound::HEADERS, logger.clone());
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

    async fn upload_blocks(&self, stream: PushStream<Block>) -> Result<(), Error> {
        let logger = self.logger.new(o!("request" => "UploadBlocks"));
        let (handle, sink) =
            intercom::stream_request(buffer_sizes::inbound::BLOCKS, logger.clone());
        let block_box = self.channels.block_box.clone();
        // TODO: make sure that a limit on the number of requests in flight
        // per service connection prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state.spawn(
            block_box
                .send(BlockMsg::NetworkBlocks(handle))
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

    async fn block_subscription(
        &self,
        stream: PushStream<Header>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let logger = self
            .subscription_logger(subscriber)
            .new(o!("stream" => "block_events"));

        let sink = BlockAnnouncementProcessor::new(
            self.channels.block_box.clone(),
            subscriber,
            self.global_state.clone(),
            logger.new(o!("direction" => "in")),
        );

        subscription::ServeBlockEvents::new(
            sink,
            self.global_state.peers.lock_server_comms(subscriber),
            logger,
        )
    }
}

#[async_trait]
impl FragmentService for NodeService {
    type GetFragmentsStream = ReplyStream<net_data::Fragment, Error>;
    type SubscriptionStream = ReplyStream<net_data::Fragment, Error>;

    async fn get_fragments(
        &self,
        ids: net_data::FragmentIds,
    ) -> Result<Self::GetFragmentsStream, Error> {
        future::err(net_error::Error::unimplemented())
    }

    async fn fragment_subscription(
        &self,
        stream: PushStream<Fragment>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let logger = self
            .subscription_logger(subscriber)
            .new(o!("stream" => "fragments"));

        let sink = FragmentProcessor::new(
            self.channels.transaction_box.clone(),
            subscriber,
            self.global_state.clone(),
            logger.new(o!("direction" => "in")),
        );

        subscription::ServeFragments::new(
            sink,
            self.global_state.peers.lock_server_comms(subscriber),
            logger,
        )
    }
}

#[async_trait]
impl GossipService for NodeService {
    type SubscriptionStream = ReplyStream<net_data::Gossip, Error>;

    async fn gossip_subscription(
        &self,
        stream: PushStream<Header>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let logger = self
            .subscription_logger(subscriber)
            .new(o!("stream" => "gossip"));

        let sink = GossipProcessor::new(
            subscriber,
            self.global_state.clone(),
            logger.new(o!("direction" => "in")),
        );

        subscription::ServeGossip::new(
            sink,
            self.global_state.peers.lock_server_comms(subscriber),
            logger,
        )
    }

    fn peers(&mut self) -> Self::PeersFuture {
        intercom::unary_future(
            self.channels.client_box.clone(),
            self.logger().new(o!("request" => "Peers")),
            ClientMsg::GetPeers,
        )
    }
}
