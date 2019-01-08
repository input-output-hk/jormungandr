use chain_core::property;

use futures::prelude::*;

use std::collections::HashMap;

// NOTE: protobuf-derived definitions used in would-be abstract core API
use super::iohk::jormungandr as gen;

pub struct ProposeTransactionsResponse<Id> {
    // TODO: define fully
    _items: HashMap<Id, gen::propose_transactions_response::Status>,
}

pub struct RecordTransactionResponse<Id> {
    // TODO: define
    _id: Id,
    _result: gen::record_transaction_response::Result,
}

pub trait Node {
    type Error: std::error::Error;
    type BlockId: property::Deserialize;
    type BlockDate;
    type Block: property::Block<Id = Self::BlockId, Date = Self::BlockDate>;
    type Header;

    type TipFuture: Future<Item = Self::BlockId, Error = Self::Error>;
    type BlocksStream: Stream<Item = Self::Block, Error = Self::Error>;
    type BlocksFuture: Future<Item = Self::BlocksStream, Error = Self::Error>;
    type HeadersStream: Stream<Item = Self::Header, Error = Self::Error>;
    type HeadersFuture: Future<Item = Self::HeadersStream, Error = Self::Error>;
    type ProposeTransactionsFuture: Future<
        Item = ProposeTransactionsResponse<Self::BlockId>,
        Error = Self::Error,
    >;
    type RecordTransactionFuture: Future<
        Item = RecordTransactionResponse<Self::BlockId>,
        Error = Self::Error,
    >;

    fn tip(&mut self) -> Self::TipFuture;
    fn stream_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::BlocksFuture;
}
