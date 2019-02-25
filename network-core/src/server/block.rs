//! Block service abstraction.

use crate::error::Code as ErrorCode;

use chain_core::property::{Block, BlockDate, BlockId, Deserialize, HasHeader, Header, Serialize};

use futures::prelude::*;

use std::{error, fmt};

/// Interface for the blockchain node service implementation responsible for
/// providing access to block data.
pub trait BlockService {
    /// The block identifier type for the blockchain.
    type BlockId: BlockId + Serialize + Deserialize;

    /// The block date type for the blockchain.
    type BlockDate: BlockDate + ToString;

    /// The type representing a block on the blockchain.
    type Block: Block<Id = Self::BlockId, Date = Self::BlockDate> + HasHeader<Header = Self::Header>;

    /// The type representing metadata header of a block.
    type Header: Header<Id = Self::BlockId, Date = Self::BlockDate> + Serialize;

    /// The type of asynchronous futures returned by method `tip`.
    ///
    /// The future resolves to the block identifier and the block date
    /// of the current chain tip as known by the serving node.
    type TipFuture: Future<Item = (Self::BlockId, Self::BlockDate), Error = BlockError>;

    /// The type of an asynchronous stream that provides blocks in
    /// response to `pull_blocks_to_*` methods.
    type PullBlocksStream: Stream<Item = Self::Block, Error = BlockError>;

    /// The type of asynchronous futures returned by `pull_blocks_to_*` methods.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type PullBlocksFuture: Future<Item = Self::PullBlocksStream, Error = BlockError>;

    /// The type of an asynchronous stream that provides block headers in
    /// response to `pull_headers_to_*` methods.
    type PullHeadersStream: Stream<Item = Self::Header, Error = BlockError>;

    /// The type of asynchronous futures returned by `pull_headers_to*` methods.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type PullHeadersFuture: Future<Item = Self::PullHeadersStream, Error = BlockError>;

    /// The type of an asynchronous stream that retrieves headers of new
    /// blocks as they are created.
    type BlockSubscription: Stream<Item = Self::Header, Error = BlockError>;

    /// The type of asynchronous futures returned by method `subscribe`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type BlockSubscriptionFuture: Future<Item = Self::BlockSubscription, Error = BlockError>;

    /// Request the current blockchain tip.
    /// The returned future resolves to the tip of the blockchain
    /// accepted by this node.
    fn tip(&mut self) -> Self::TipFuture;

    /// Get blocks, walking forward in a range between either of the given
    /// starting points, and the ending point.
    fn pull_blocks_to(
        &mut self,
        from: &[Self::BlockId],
        to: &Self::BlockId,
    ) -> Self::PullBlocksFuture;

    // Stream blocks from either of the given starting points
    // to the server's tip.
    fn pull_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::PullBlocksFuture;

    /// Get block headers, walking forward in a range between any of the given
    /// starting points, and the ending point.
    fn pull_headers_to(
        &mut self,
        from: &[Self::BlockId],
        to: &Self::BlockId,
    ) -> Self::PullHeadersFuture;

    // Stream block headers from either of the given starting points
    // to the server's tip.
    fn pull_headers_to_tip(&mut self, from: &[Self::BlockId]) -> Self::PullHeadersFuture;

    // Returns a future that resolves to an asynchronous subscription stream
    // that can be used to notify of newly created blocks.
    fn subscribe(&mut self) -> Self::BlockSubscriptionFuture;
}

/// Represents errors that can be returned by the block service.
#[derive(Debug)]
pub struct BlockError(pub ErrorCode);

impl error::Error for BlockError {}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "block service error: {}", self.0)
    }
}
