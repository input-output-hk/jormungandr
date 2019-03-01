use super::ConnectionState;
use crate::blockcfg::BlockConfig;
use crate::intercom::{
    self, stream_reply, subscription_reply, unary_reply, BlockMsg, ClientMsg, ReplyFuture,
    ReplyStream, SubscriptionFuture, SubscriptionStream, TransactionMsg,
};
use crate::utils::task::TaskMessageBox;

use network_core::server::{
    block::{BlockError, BlockService},
    transaction::{
        ProposeTransactionsResponse, RecordTransactionResponse, TransactionError,
        TransactionService,
    },
    Node,
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

    fn block_service(&self) -> Option<Self::BlockService> {
        Some(ConnectionBlockService::new(&self.state))
    }

    fn transaction_service(&self) -> Option<Self::TransactionService> {
        // Not implemented yet
        None
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

    fn pull_headers_to_tip(&mut self, from: &[Self::BlockId]) -> Self::PullHeadersFuture {
        unimplemented!()
    }

    fn subscribe(&mut self) -> Self::BlockSubscriptionFuture {
        let (handle, future) = subscription_reply();
        self.block_box.send_to(BlockMsg::Subscribe(handle));
        future
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
    type RecordTransactionFuture =
        ReplyFuture<RecordTransactionResponse<B::TransactionId>, TransactionError>;
    type GetTransactionsStream = ReplyStream<Self::Transaction, TransactionError>;
    type GetTransactionsFuture = ReplyFuture<Self::GetTransactionsStream,TransactionError>;

    fn propose_transactions(
        &mut self,
        _ids: &[Self::TransactionId],
    ) -> Self::ProposeTransactionsFuture {
        unimplemented!()
    }

    fn get_transactions(
        &mut self,
        _ids: &[Self::TransactionId],
    ) -> Self::GetTransactionsFuture {
        unimplemented!()
    }
}
