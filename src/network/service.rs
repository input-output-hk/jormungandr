use super::ConnectionState;
use crate::blockcfg::BlockConfig;
use crate::intercom::{
    self, stream_reply, unary_reply, ClientMsg, ReplyFuture, ReplyStream, TransactionMsg,
};
use crate::utils::task::TaskMessageBox;

use chain_core::property::Header;
use network_core::server::{
    block::{BlockError, BlockService, HeaderService},
    transaction::{
        ProposeTransactionsResponse, RecordTransactionResponse, TransactionError,
        TransactionService,
    },
    Node,
};

use futures::future::{self, FutureResult};
use futures::prelude::*;

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
    type HeaderService = ConnectionBlockService<B>;
    type TransactionService = ConnectionTransactionService<B>;

    fn block_service(&self) -> Option<Self::BlockService> {
        Some(ConnectionBlockService::new(&self.state))
    }

    fn header_service(&self) -> Option<Self::HeaderService> {
        Some(ConnectionBlockService::new(&self.state))
    }

    fn transaction_service(&self) -> Option<Self::TransactionService> {
        // Not implemented yet
        None
    }
}

impl From<intercom::Error> for BlockError {
    fn from(err: intercom::Error) -> Self {
        BlockError(err.code())
    }
}

pub struct ConnectionBlockService<B: BlockConfig> {
    pub client_box: TaskMessageBox<ClientMsg<B>>,
}

impl<B: BlockConfig> ConnectionBlockService<B> {
    pub fn new(conn: &ConnectionState<B>) -> Self {
        ConnectionBlockService {
            client_box: conn.channels.client_box.clone(),
        }
    }
}

impl<B: BlockConfig> Clone for ConnectionBlockService<B> {
    fn clone(&self) -> Self {
        ConnectionBlockService {
            client_box: self.client_box.clone(),
        }
    }
}

impl<B: BlockConfig> BlockService for ConnectionBlockService<B> {
    type BlockId = B::BlockHash;
    type BlockDate = B::BlockDate;
    type Block = B::Block;
    type TipFuture = futures::Map<
        ReplyFuture<B::BlockHeader, BlockError>,
        fn(B::BlockHeader) -> (B::BlockHash, B::BlockDate),
    >;
    type GetBlocksStream = ReplyStream<B::Block, BlockError>;
    type GetBlocksFuture = FutureResult<Self::GetBlocksStream, BlockError>;
    type PullBlocksToTipStream = ReplyStream<B::Block, BlockError>;
    type PullBlocksFuture = FutureResult<Self::PullBlocksToTipStream, BlockError>;

    fn tip(&mut self) -> Self::TipFuture {
        let (handle, future) = unary_reply();
        self.client_box.send_to(ClientMsg::GetBlockTip(handle));
        future.map(|header| (header.id(), header.date()))
    }

    fn pull_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::PullBlocksFuture {
        let (handle, stream) = stream_reply();
        self.client_box
            .send_to(ClientMsg::PullBlocksToTip(from.into(), handle));
        future::ok(stream)
    }

    fn pull_blocks_to(
        &mut self,
        from: &[Self::BlockId],
        to: &Self::BlockId,
    ) -> Self::PullBlocksFuture {
        unimplemented!()
    }
}

impl<B: BlockConfig> HeaderService for ConnectionBlockService<B> {
    type Header = B::BlockHeader;
    type HeaderId = B::BlockHash;
    type GetHeadersStream = ReplyStream<B::BlockHeader, BlockError>;
    type GetHeadersFuture = FutureResult<Self::GetHeadersStream, BlockError>;

    fn block_headers(
        &mut self,
        from: &[Self::HeaderId],
        to: &Self::HeaderId,
    ) -> Self::GetHeadersFuture {
        unimplemented!()
    }

    fn block_headers_to_tip(&mut self, from: &[Self::HeaderId]) -> Self::GetHeadersFuture {
        unimplemented!()
    }
}

impl From<intercom::Error> for TransactionError {
    fn from(err: intercom::Error) -> Self {
        TransactionError(err.code())
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
    type TransactionId = B::TransactionId;
    type ProposeTransactionsFuture =
        ReplyFuture<ProposeTransactionsResponse<B::TransactionId>, TransactionError>;
    type RecordTransactionFuture =
        ReplyFuture<RecordTransactionResponse<B::TransactionId>, TransactionError>;

    fn propose_transactions(
        &mut self,
        ids: &[Self::TransactionId],
    ) -> Self::ProposeTransactionsFuture {
        unimplemented!()
    }
}
