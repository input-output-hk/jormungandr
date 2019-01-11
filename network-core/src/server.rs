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

/// Interface to application logic of the blockchain node server.
///
/// An implementation of a blockchain node implements this trait to
/// serve the network protocols using node's subsystems such as
/// block storage and transaction engine.
pub trait Node {
    /// The block identifier type for the blockchain.
    type BlockId: property::BlockId;

    /// The block date type for the blockchain.
    type BlockDate: property::BlockDate;

    /// The type representing a block on the blockchain.
    type Block: property::Block<Id = Self::BlockId, Date = Self::BlockDate>;

    /// The type representing metadata header of a block.
    /// If the blockchain does not feature headers, this can be the unit type
    /// `()`.
    type Header: property::Header;

    /// The type of asynchronous futures returned by method `tip`.
    ///
    /// The future resolves to the block identifier and the block date
    /// of the current chain tip as known by the serving node.
    type TipFuture: Future<Item = (Self::BlockId, Self::BlockDate), Error = Error>;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `get_blocks`.
    type GetBlocksStream: Stream<Item = Self::Block, Error = Error>;

    /// The type of asynchronous futures returned by method `get_blocks`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetBlocksFuture: Future<Item = Self::GetBlocksStream, Error = Error>;

    /// The type of an asynchronous stream that provides block headers in
    /// response to method `get_headers`.
    type GetHeadersStream: Stream<Item = Self::Header, Error = Error>;

    /// The type of asynchronous futures returned by method `get_headers`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetHeadersFuture: Future<Item = Self::GetHeadersStream, Error = Error>;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `stream_blocks_to_tip`.
    type StreamBlocksToTipStream: Stream<Item = Self::Block, Error = Error>;

    /// The type of asynchronous futures returned by method `stream_blocks_to_tip`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type StreamBlocksToTipFuture: Future<Item = Self::StreamBlocksToTipStream, Error = Error>;

    /// The type of asynchronous futures returned by method `propose_transactions`.
    type ProposeTransactionsFuture: Future<
        Item = ProposeTransactionsResponse<Self::BlockId>,
        Error = Error,
    >;

    /// The type of asynchronous futures returned by method `record_transaction`.
    type RecordTransactionFuture: Future<
        Item = RecordTransactionResponse<Self::BlockId>,
        Error = Error,
    >;

    fn tip(&mut self) -> Self::TipFuture;
    fn stream_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::StreamBlocksToTipFuture;
}
