//! Abstractions for the network server-side interface of a blockchain node.

use chain_core::property;

use futures::prelude::*;

use std::fmt;

use super::codes;

/// Represents errors that can be returned by the node implementation.
#[derive(Debug)]
pub struct Error(); // TODO: define specific error variants and details

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unknown network node error")
    }
}

pub struct ProposeTransactionsResponse<Id> {
    // TODO: define fully
    _items: Vec<(Id, codes::TransactionStatus)>,
}

pub struct RecordTransactionResponse<Id> {
    // TODO: define
    _id: Id,
    _result: codes::TransactionAcceptance,
}

pub trait Node {
    type BlockId: property::BlockId;
    type BlockDate: property::BlockDate;
    type Block: property::Block<Id = Self::BlockId, Date = Self::BlockDate>;
    type Header: property::Header;

    type TipFuture: Future<Item = (Self::BlockId, Self::BlockDate), Error = Error>;
    type GetBlocksStream: Stream<Item = Self::Block, Error = Error>;
    type GetBlocksFuture: Future<Item = Self::GetBlocksStream, Error = Error>;
    type GetHeadersStream: Stream<Item = Self::Header, Error = Error>;
    type GetHeadersFuture: Future<Item = Self::GetHeadersStream, Error = Error>;
    type StreamBlocksToTipStream: Stream<Item = Self::Block, Error = Error>;
    type StreamBlocksToTipFuture: Future<Item = Self::StreamBlocksToTipStream, Error = Error>;
    type ProposeTransactionsFuture: Future<
        Item = ProposeTransactionsResponse<Self::BlockId>,
        Error = Error,
    >;
    type RecordTransactionFuture: Future<
        Item = RecordTransactionResponse<Self::BlockId>,
        Error = Error,
    >;

    fn tip(&mut self) -> Self::TipFuture;
    fn stream_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::StreamBlocksToTipFuture;
}
