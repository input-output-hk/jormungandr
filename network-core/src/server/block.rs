//! Block service abstraction.

use chain_core::property::{Block, BlockDate, BlockId, Deserialize, Header, Serialize};

use futures::prelude::*;

use std::fmt;

/// Interface for the blockchain node service implementation responsible for
/// providing access to blocks.
pub trait BlockService {
    /// The block identifier type for the blockchain.
    type BlockId: BlockId + Serialize + Deserialize;

    /// The block date type for the blockchain.
    type BlockDate: BlockDate + ToString;

    /// The type representing a block on the blockchain.
    type Block: Block<Id = Self::BlockId, Date = Self::BlockDate>;

    /// The type of asynchronous futures returned by method `tip`.
    ///
    /// The future resolves to the block identifier and the block date
    /// of the current chain tip as known by the serving node.
    type TipFuture: Future<Item = (Self::BlockId, Self::BlockDate), Error = BlockError>;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `get_blocks`.
    type GetBlocksStream: Stream<Item = Self::Block, Error = BlockError>;

    /// The type of asynchronous futures returned by method `get_blocks`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetBlocksFuture: Future<Item = Self::GetBlocksStream, Error = BlockError>;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `stream_blocks_to_tip`.
    type StreamBlocksToTipStream: Stream<Item = Self::Block, Error = BlockError>;

    /// The type of asynchronous futures returned by method `stream_blocks_to_tip`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type StreamBlocksFuture: Future<Item = Self::StreamBlocksToTipStream, Error = BlockError>;

    fn tip(&mut self) -> Self::TipFuture;
    fn stream_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::StreamBlocksFuture;

    // TODO: return a stream instead of the vector.
    /// Get block headers between two dates.
    fn block_headers(
        &mut self,
        from: &[Self::BlockId],
        to: &Self::BlockId,
    ) -> Self::GetHeadersFuture;
    // Stream blocks to the provided tip.
    fn stream_blocks_to(
        &mut self,
        from: &[Self::BlockId],
        to: &Self::BlockId,
    ) -> Self::StreamBlocksFuture;
    fn block_headers_to_tip(&mut self, from: &[Self::BlockId]) -> Self::GetHeadersFuture;
}

/// Interface for the blockchain node service implementation responsible for
/// providing access to block headers.
pub trait HeaderService {
    /// The type representing metadata header of a block.
    type Header: Header + Serialize;

    /// The type of an asynchronous stream that provides block headers in
    /// response to method `get_headers`.
    type GetHeadersStream: Stream<Item = Self::Header, Error = BlockError>;

    /// The type of asynchronous futures returned by method `get_headers`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetHeadersFuture: Future<Item = Self::GetHeadersStream, Error = BlockError>;
}

/// Represents errors that can be returned by the node service implementation.
#[derive(Debug)]
pub struct BlockError(); // TODO: define specific error variants and details

impl std::error::Error for BlockError {}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unknown block service error")
    }
}
