use super::ConnectionState;
use crate::blockcfg::{Block, BlockDate, Header, HeaderHash, Message, MessageId};
use crate::intercom::{
    self, stream_reply, subscription_reply, unary_reply, BlockMsg, ClientMsg, ReplyFuture,
    ReplyStream, SubscriptionFuture, SubscriptionStream, TransactionMsg,
};
use crate::utils::task::TaskMessageBox;
use std::collections::BTreeMap;
use std::marker::PhantomData;

use network::p2p_topology::{self as p2p, Gossip, Id, P2pTopology};
use network_core::{
    error as core_error, gossip,
    server::{
        block::BlockService,
        gossip::GossipService,
        transaction::{ProposeTransactionsResponse, TransactionService},
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
    type TransactionService = ConnectionTransactionService;
    type GossipService = ConnectionGossipService;

    fn block_service(&self) -> Option<Self::BlockService> {
        Some(ConnectionBlockService::new(&self.state))
    }

    fn transaction_service(&self) -> Option<Self::TransactionService> {
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

pub struct ConnectionTransactionService {
    transaction_box: TaskMessageBox<TransactionMsg>,
}

impl Clone for ConnectionTransactionService {
    fn clone(&self) -> Self {
        ConnectionTransactionService {
            transaction_box: self.transaction_box.clone(),
        }
    }
}

impl TransactionService for ConnectionTransactionService {
    type Transaction = Message;
    type TransactionId = MessageId;
    type ProposeTransactionsFuture =
        ReplyFuture<ProposeTransactionsResponse<MessageId>, core_error::Error>;
    type GetTransactionsStream = ReplyStream<Self::Transaction, core_error::Error>;
    type GetTransactionsFuture = ReplyFuture<Self::GetTransactionsStream, core_error::Error>;
    type TransactionSubscription = SubscriptionStream<Message>;
    type TransactionSubscriptionFuture = SubscriptionFuture<Message>;

    fn propose_transactions(
        &mut self,
        _ids: &[Self::TransactionId],
    ) -> Self::ProposeTransactionsFuture {
        unimplemented!()
    }

    fn get_transactions(&mut self, _ids: &[Self::TransactionId]) -> Self::GetTransactionsFuture {
        unimplemented!()
    }

    fn transaction_subscription<Out>(
        &mut self,
        _outbound: Out,
    ) -> Self::TransactionSubscriptionFuture
    where
        Out: Stream<Item = Self::Transaction, Error = core_error::Error>,
    {
        unimplemented!()
    }
}

pub struct ConnectionGossipService {
    p2p: P2pTopology,
    node: poldercast::Node,
    _phantom: PhantomData<Gossip>,
}

impl GossipService for ConnectionGossipService {
    type Message = Gossip;
    type MessageFuture = future::FutureResult<(gossip::NodeId, Self::Message), core_error::Error>;

    /// Record and process gossip event.
    fn record_gossip(
        &mut self,
        node_id: gossip::NodeId,
        gossip: &Self::Message,
    ) -> Self::MessageFuture {
        let nodes: BTreeMap<_, _> = (&gossip.0)
            .into_iter()
            .map(|node| (*node.id(), node.clone()))
            .collect();
        let node_id: Id = p2p::from_node_id(&node_id);
        if let Some(them) = nodes.get(&node_id).cloned() {
            self.p2p.update(nodes);
            let reply_nodes = self.p2p.select_gossips(&them);
            let reply = gossip::Gossip::from_nodes(reply_nodes.into_iter().map(|(_, node)| node));
            let node_id = p2p::to_node_id(self.node.id());
            future::ok((node_id, reply))
        } else {
            future::err(core_error::Error::new(
                core_error::Code::Internal,
                "No message",
            ))
        }
    }
}

impl ConnectionGossipService {
    fn new(state: &ConnectionState) -> Self {
        ConnectionGossipService {
            p2p: state.topology.clone(),
            node: state.node.clone(),
            _phantom: PhantomData,
        }
    }
}

impl Clone for ConnectionGossipService {
    fn clone(&self) -> Self {
        ConnectionGossipService {
            _phantom: PhantomData,
            node: self.node.clone(),
            p2p: self.p2p.clone(),
        }
    }
}
