use super::ConnectionState;
use crate::blockcfg::BlockConfig;
use crate::intercom::{
    self, stream_reply, subscription_reply, unary_reply, BlockMsg, ClientMsg, ReplyFuture,
    ReplyStream, SubscriptionFuture, SubscriptionStream, TransactionMsg,
};
use crate::utils::task::TaskMessageBox;
use std::collections::BTreeMap;
use std::marker::PhantomData;

use network::p2p_topology::{self as p2p, Gossip, Id, P2pTopology};
use network_core::{
    gossip,
    server::{
        block::{BlockError, BlockService},
        gossip::{GossipError, GossipService},
        transaction::{ProposeTransactionsResponse, TransactionError, TransactionService},
        Node,
    },
};

use futures::future::{self, FutureResult};

pub struct ConnectionServices<B: BlockConfig> {
    state: ConnectionState<B>,
}

impl<B: BlockConfig> ConnectionServices<B> {
    pub fn new(state: ConnectionState<B>) -> Self {
        ConnectionServices { state }
    }
}

impl<B: BlockConfig> Node for ConnectionServices<B> {
    type BlockService = ConnectionBlockService<B>;
    type TransactionService = ConnectionTransactionService<B>;
    type GossipService = ConnectionGossipService<B>;

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

impl From<intercom::Error> for BlockError {
    fn from(err: intercom::Error) -> Self {
        BlockError::with_code_and_cause(err.code(), err)
    }
}

pub struct ConnectionBlockService<B: BlockConfig> {
    client_box: TaskMessageBox<ClientMsg<B>>,
    block_box: TaskMessageBox<BlockMsg<B>>,
}

impl<B: BlockConfig> ConnectionBlockService<B> {
    pub fn new(conn: &ConnectionState<B>) -> Self {
        ConnectionBlockService {
            client_box: conn.channels.client_box.clone(),
            block_box: conn.channels.block_box.clone(),
        }
    }
}

impl<B: BlockConfig> Clone for ConnectionBlockService<B> {
    fn clone(&self) -> Self {
        ConnectionBlockService {
            client_box: self.client_box.clone(),
            block_box: self.block_box.clone(),
        }
    }
}

impl<B: BlockConfig> BlockService for ConnectionBlockService<B> {
    type BlockId = B::BlockHash;
    type BlockDate = B::BlockDate;
    type Block = B::Block;
    type TipFuture = ReplyFuture<B::BlockHeader, BlockError>;
    type Header = B::BlockHeader;
    type PullBlocksStream = ReplyStream<B::Block, BlockError>;
    type PullBlocksFuture = FutureResult<Self::PullBlocksStream, BlockError>;
    type GetBlocksStream = ReplyStream<B::Block, BlockError>;
    type GetBlocksFuture = FutureResult<Self::GetBlocksStream, BlockError>;
    type PullHeadersStream = ReplyStream<B::BlockHeader, BlockError>;
    type PullHeadersFuture = FutureResult<Self::PullHeadersStream, BlockError>;
    type GetHeadersStream = ReplyStream<B::BlockHeader, BlockError>;
    type GetHeadersFuture = FutureResult<Self::GetHeadersStream, BlockError>;
    type BlockSubscription = SubscriptionStream<B::BlockHeader, BlockError>;
    type BlockSubscriptionFuture = SubscriptionFuture<B::BlockHeader, BlockError>;
    type AnnounceBlockFuture = ReplyFuture<(), BlockError>;

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

    fn subscribe(&mut self) -> Self::BlockSubscriptionFuture {
        let (handle, future) = subscription_reply();
        self.block_box.send_to(BlockMsg::Subscribe(handle));
        future
    }

    fn announce_block(&mut self, _header: &Self::Header) -> Self::AnnounceBlockFuture {
        unimplemented!()
    }
}

impl From<intercom::Error> for TransactionError {
    fn from(err: intercom::Error) -> Self {
        TransactionError::with_code_and_cause(err.code(), err)
    }
}

pub struct ConnectionTransactionService<B: BlockConfig> {
    transaction_box: TaskMessageBox<TransactionMsg<B>>,
}

impl<B: BlockConfig> Clone for ConnectionTransactionService<B> {
    fn clone(&self) -> Self {
        ConnectionTransactionService {
            transaction_box: self.transaction_box.clone(),
        }
    }
}

impl<B: BlockConfig> TransactionService for ConnectionTransactionService<B> {
    type Transaction = B::Transaction;
    type TransactionId = B::TransactionId;
    type ProposeTransactionsFuture =
        ReplyFuture<ProposeTransactionsResponse<B::TransactionId>, TransactionError>;
    type GetTransactionsStream = ReplyStream<Self::Transaction, TransactionError>;
    type GetTransactionsFuture = ReplyFuture<Self::GetTransactionsStream, TransactionError>;
    type AnnounceTransactionFuture = ReplyFuture<(), TransactionError>;

    fn propose_transactions(
        &mut self,
        _ids: &[Self::TransactionId],
    ) -> Self::ProposeTransactionsFuture {
        unimplemented!()
    }

    fn get_transactions(&mut self, _ids: &[Self::TransactionId]) -> Self::GetTransactionsFuture {
        unimplemented!()
    }

    fn announce_transaction(
        &mut self,
        _tx: &[Self::TransactionId],
    ) -> Self::AnnounceTransactionFuture {
        unimplemented!()
    }
}

pub struct ConnectionGossipService<B: BlockConfig> {
    p2p: P2pTopology,
    node: poldercast::Node,
    _phantom: PhantomData<B::Gossip>,
}

impl<B: BlockConfig> GossipService for ConnectionGossipService<B> {
    type Message = Gossip;
    type MessageFuture = future::FutureResult<(gossip::NodeId, Self::Message), GossipError>;

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
            future::err(GossipError::failed("No message"))
        }
    }
}

impl<B: BlockConfig> ConnectionGossipService<B> {
    fn new(state: &ConnectionState<B>) -> Self {
        ConnectionGossipService {
            p2p: state.topology.clone(),
            node: state.node.clone(),
            _phantom: PhantomData,
        }
    }
}

impl From<intercom::Error> for GossipError {
    fn from(err: intercom::Error) -> Self {
        GossipError::with_code_and_cause(err.code(), err)
    }
}

impl<B: BlockConfig> Clone for ConnectionGossipService<B> {
    fn clone(&self) -> Self {
        ConnectionGossipService {
            _phantom: PhantomData,
            node: self.node.clone(),
            p2p: self.p2p.clone(),
        }
    }
}
