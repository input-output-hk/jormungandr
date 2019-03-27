use super::ConnectionState;
use crate::blockcfg::{Block, BlockDate, Header, HeaderHash, Message, MessageId};
use crate::intercom::{
    self, stream_reply, subscription_reply, unary_reply, BlockMsg, ClientMsg, ReplyFuture,
    ReplyStream, SubscriptionFuture, SubscriptionStream, TransactionMsg,
};
use crate::utils::task::TaskMessageBox;

use network::p2p_topology::{self as p2p, P2pTopology};
use network_core::{
    error as core_error,
    gossip::Gossip,
    server::{
        block::BlockService,
        content::{ContentService, ProposeTransactionsResponse},
        gossip::GossipService,
        Node,
    },
};

use futures::future::{self, FutureResult};
use futures::prelude::*;

pub struct ConnectionServices {
    state: ConnectionState,
}

impl ConnectionServices {
    pub fn new(state: ConnectionState) -> Self {
        ConnectionServices { state }
    }
}

impl Node for ConnectionServices {
    type BlockService = ConnectionBlockService;
    type ContentService = ConnectionContentService;
    type GossipService = ConnectionGossipService;

    fn block_service(&self) -> Option<Self::BlockService> {
        Some(ConnectionBlockService::new(&self.state))
    }

    fn content_service(&self) -> Option<Self::ContentService> {
        // Not implemented yet
        None
    }

    fn gossip_service(&self) -> Option<Self::GossipService> {
        Some(ConnectionGossipService::new(&self.state))
    }
}

impl From<intercom::Error> for core_error::Error {
    fn from(err: intercom::Error) -> Self {
        core_error::Error::new(err.code(), err)
    }
}

pub struct ConnectionBlockService {
    client_box: TaskMessageBox<ClientMsg>,
    block_box: TaskMessageBox<BlockMsg>,
}

impl ConnectionBlockService {
    pub fn new(conn: &ConnectionState) -> Self {
        ConnectionBlockService {
            client_box: conn.channels.client_box.clone(),
            block_box: conn.channels.block_box.clone(),
        }
    }
}

impl Clone for ConnectionBlockService {
    fn clone(&self) -> Self {
        ConnectionBlockService {
            client_box: self.client_box.clone(),
            block_box: self.block_box.clone(),
        }
    }
}

impl BlockService for ConnectionBlockService {
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
    type BlockSubscription = SubscriptionStream<Header>;
    type BlockSubscriptionFuture = SubscriptionFuture<Header>;

    fn tip(&mut self) -> Self::TipFuture {
        let (handle, future) = unary_reply();
        self.client_box.send_to(ClientMsg::GetBlockTip(handle));
        future
    }

    fn pull_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::PullBlocksFuture {
        let (handle, stream) = stream_reply();
        self.client_box
            .send_to(ClientMsg::PullBlocksToTip(from.into(), handle));
        future::ok(stream)
    }

    fn get_blocks(&mut self, ids: &[Self::BlockId]) -> Self::GetBlocksFuture {
        let (handle, stream) = stream_reply();
        self.client_box
            .send_to(ClientMsg::GetBlocks(ids.into(), handle));
        future::ok(stream)
    }

    fn get_headers(&mut self, ids: &[Self::BlockId]) -> Self::GetHeadersFuture {
        let (handle, stream) = stream_reply();
        self.client_box
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

    fn block_subscription<Out>(&mut self, outbound: Out) -> Self::BlockSubscriptionFuture
    where
        Out: Stream<Item = Self::Header, Error = core_error::Error>,
    {
        // FIXME: plug in outbound stream
        let (handle, future) = subscription_reply();
        self.block_box.send_to(BlockMsg::Subscribe(handle));
        future
    }
}

pub struct ConnectionContentService {
    transaction_box: TaskMessageBox<TransactionMsg>,
}

impl Clone for ConnectionContentService {
    fn clone(&self) -> Self {
        ConnectionContentService {
            transaction_box: self.transaction_box.clone(),
        }
    }
}

impl ContentService for ConnectionContentService {
    type Message = Message;
    type MessageId = MessageId;
    type ProposeTransactionsFuture =
        ReplyFuture<ProposeTransactionsResponse<MessageId>, core_error::Error>;
    type GetMessagesStream = ReplyStream<Self::Message, core_error::Error>;
    type GetMessagesFuture = ReplyFuture<Self::GetMessagesStream, core_error::Error>;
    type MessageSubscription = SubscriptionStream<Message>;
    type MessageSubscriptionFuture = SubscriptionFuture<Message>;

    fn propose_transactions(
        &mut self,
        _ids: &[Self::MessageId],
    ) -> Self::ProposeTransactionsFuture {
        unimplemented!()
    }

    fn get_messages(&mut self, _ids: &[Self::MessageId]) -> Self::GetMessagesFuture {
        unimplemented!()
    }

    fn message_subscription<S>(&mut self, _inbound: S) -> Self::MessageSubscriptionFuture
    where
        S: Stream<Item = Self::Message, Error = core_error::Error>,
    {
        unimplemented!()
    }
}

pub struct ConnectionGossipService {
    p2p: P2pTopology,
    node: p2p::Node,
}

impl GossipService for ConnectionGossipService {
    type Node = p2p::Node;
    type GossipSubscription = SubscriptionStream<Gossip<p2p::Node>>;
    type GossipSubscriptionFuture = SubscriptionFuture<Gossip<p2p::Node>>;

    fn gossip_subscription<In>(&mut self, inbound: In) -> Self::GossipSubscriptionFuture
    where
        In: Stream<Item = Gossip<p2p::Node>, Error = core_error::Error>,
    {
        inbound.for_each(|gossip| {
            self.p2p.update(gossip.into_nodes());
            future::ok(())
        });
        // TODO: send periodic updates to nodes
        unimplemented!()
    }
}

impl ConnectionGossipService {
    fn new(state: &ConnectionState) -> Self {
        ConnectionGossipService {
            p2p: state.topology.clone(),
            node: state.node.clone(),
        }
    }
}

impl Clone for ConnectionGossipService {
    fn clone(&self) -> Self {
        ConnectionGossipService {
            node: self.node.clone(),
            p2p: self.p2p.clone(),
        }
    }
}
