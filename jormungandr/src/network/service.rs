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
use chain_network::data::{
    Block, BlockId, BlockIds, Fragment, FragmentId, FragmentIds, Gossip, Header, Peer, Peers,
};
use chain_network::error::{self as net_error, Error};

use async_trait::async_trait;
use futures03::prelude::*;
use slog::Logger;

use std::convert::TryFrom;

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
    fn subscription_logger(&self, subscriber: Peer, stream_name: &'static str) -> Logger {
        self.logger
            .new(o!("peer" => subscriber.to_string(), "stream" => stream_name))
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

async fn send_message<T>(mbox: MessageBox<T>, msg: T, logger: Logger) -> Result<(), Error> {
    mbox.send(msg).await.map_err(|e| {
        error!(
            logger,
            "failed to enqueue message for processing";
            "reason" => %e,
        );
        Error::new(net_error::Code::Internal, e)
    })
}

#[async_trait]
impl BlockService for NodeService {
    type PullBlocksToTipStream = ResponseStream<app_data::Block>;
    type GetBlocksStream = ResponseStream<app_data::Block>;
    type PullHeadersStream = ResponseStream<app_data::Header>;
    type GetHeadersStream = ResponseStream<app_data::Header>;
    type SubscriptionStream = BlockEventSubscription;

    fn block0(&mut self) -> BlockId {
        BlockId::try_from(self.global_state.block0_hash.as_bytes()).unwrap()
    }

    async fn tip(&self) -> Result<Header, Error> {
        let logger = self.logger().new(o!("request" => "Tip"));
        let (reply_handle, reply_future) = intercom::unary_reply(logger.clone());
        let mbox = self.channels.client_box.clone();
        send_message(mbox, ClientMsg::GetBlockTip(reply_handle), logger).await?;
        let header = reply_future.await?;
        Ok(header.encode())
    }

    async fn pull_blocks_to_tip(
        &self,
        from: BlockIds,
    ) -> Result<Self::PullBlocksToTipStream, Error> {
        let from = from.decode()?;
        let logger = self.logger().new(o!("request" => "PullBlocksToTip"));
        let (handle, stream) =
            intercom::stream_reply(buffer_sizes::outbound::BLOCKS, logger.clone());
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::PullBlocksToTip(from, handle), logger).await?;
        Ok(convert::response_stream(stream))
    }

    async fn get_blocks(&self, ids: BlockIds) -> Result<Self::GetBlocksStream, Error> {
        let ids = ids.decode()?;
        let logger = self.logger().new(o!("request" => "GetBlocks"));
        let (handle, stream) =
            intercom::stream_reply(buffer_sizes::outbound::BLOCKS, logger.clone());
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::GetBlocks(ids, handle), logger).await?;
        Ok(convert::response_stream(stream))
    }

    async fn get_headers(&self, ids: BlockIds) -> Result<Self::GetHeadersStream, Error> {
        let ids = ids.decode()?;
        let logger = self.logger().new(o!("request" => "GetHeaders"));
        let (handle, stream) =
            intercom::stream_reply(buffer_sizes::outbound::HEADERS, logger.clone());
        let client_box = self.channels.client_box.clone();
        send_message(client_box, ClientMsg::GetHeaders(ids, handle), logger).await?;
        Ok(convert::response_stream(stream))
    }

    async fn pull_headers(
        &self,
        from: BlockIds,
        to: BlockId,
    ) -> Result<Self::PullHeadersStream, Error> {
        let from = from.decode()?;
        let to = to.decode()?;
        let logger = self.logger().new(o!("request" => "PullHeaders"));
        let (handle, stream) =
            intercom::stream_reply(buffer_sizes::outbound::HEADERS, logger.clone());
        let client_box = self.channels.client_box.clone();
        send_message(
            client_box,
            ClientMsg::GetHeadersRange(from, to, handle),
            logger,
        )
        .await?;
        Ok(convert::response_stream(stream))
    }

    async fn push_headers(&self, stream: PushStream<Header>) -> Result<(), Error> {
        let logger = self.logger.new(o!("request" => "PushHeaders"));
        let (handle, sink) =
            intercom::stream_request(buffer_sizes::inbound::HEADERS, logger.clone());
        let block_box = self.channels.block_box.clone();
        send_message(block_box, BlockMsg::ChainHeaders(handle), logger).await?;
        stream
            .and_then(|header| async { header.decode() })
            .forward(sink.sink_err_into())
            .await
    }

    async fn upload_blocks(&self, stream: PushStream<Block>) -> Result<(), Error> {
        let logger = self.logger.new(o!("request" => "UploadBlocks"));
        let (handle, sink) =
            intercom::stream_request(buffer_sizes::inbound::BLOCKS, logger.clone());
        let block_box = self.channels.block_box.clone();
        send_message(block_box, BlockMsg::NetworkBlocks(handle), logger).await?;
        stream
            .and_then(|block| async { block.decode() })
            .forward(sink.sink_err_into())
            .await
    }

    async fn block_subscription(
        &self,
        subscriber: Peer,
        stream: PushStream<Header>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let logger = self.subscription_logger(subscriber, "block_events");
        let subscriber = Address::new(subscriber.addr()).unwrap();

        self.global_state
            .spawn(subscription::process_block_announcements(
                stream,
                self.channels.block_box.clone(),
                subscriber,
                self.global_state.clone(),
                logger.new(o!("direction" => "in")),
            ));

        let outbound = self
            .global_state
            .peers
            .subscribe_to_block_events(subscriber)
            .await;
        Ok(outbound)
    }
}

#[async_trait]
impl FragmentService for NodeService {
    type GetFragmentsStream = ResponseStream<app_data::Fragment>;
    type SubscriptionStream = FragmentSubscription;

    async fn get_fragments(&self, ids: FragmentIds) -> Result<Self::GetFragmentsStream, Error> {
        Err(net_error::Error::unimplemented())
    }

    async fn fragment_subscription(
        &self,
        subscriber: Peer,
        stream: PushStream<Fragment>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let logger = self.subscription_logger(subscriber, "fragments");
        let subscriber = Address::new(subscriber.addr()).unwrap();

        self.global_state.spawn(subscription::process_fragments(
            stream,
            self.channels.transaction_box.clone(),
            subscriber,
            self.global_state.clone(),
            logger.new(o!("direction" => "in")),
        ));

        let outbound = self
            .global_state
            .peers
            .subscribe_to_fragments(subscriber)
            .await;
        Ok(outbound)
    }
}

#[async_trait]
impl GossipService for NodeService {
    type SubscriptionStream = GossipSubscription;

    async fn gossip_subscription(
        &self,
        subscriber: Peer,
        stream: PushStream<Gossip>,
    ) -> Result<Self::SubscriptionStream, Error> {
        let logger = self.subscription_logger(subscriber, "gossip");
        let subscriber = Address::new(subscriber.addr()).unwrap();

        self.global_state.spawn(subscription::process_gossip(
            stream,
            subscriber,
            self.global_state.clone(),
            logger.new(o!("direction" => "in")),
        ));

        let outbound = self
            .global_state
            .peers
            .subscribe_to_gossip(subscriber)
            .await;
        Ok(outbound)
    }

    async fn peers(&mut self) -> Result<Peers, Error> {
        let logger = self.logger().new(o!("request" => "Peers"));
        let topology = &self.global_state.topology;
        let view = topology.view(poldercast::Selection::Any).await;
        let mut peers = Vec::new();
        for n in view.peers.into_iter() {
            if let Some(addr) = n.to_socketaddr() {
                peers.push(Peer { addr });
            }
        }
        if peers.len() == 0 {
            // No peers yet, put self as the peer to bootstrap from
            if let Some(addr) = view.self_node.address().and_then(|x| x.to_socketaddr()) {
                peers.push(Peer { addr });
            }
        }
        Ok(peers.into_boxed_slice())
    }
}
