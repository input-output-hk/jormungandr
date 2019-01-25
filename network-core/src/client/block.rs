use super::Error;

use chain_core::property::{Block, HasHeader};

use futures::prelude::*;

/// Interface for the blockchain node service responsible for
/// providing access to blocks.
pub trait BlockService<T: Block> {
    /// The type of asynchronous futures returned by method `tip`.
    ///
    /// The future resolves to the block identifier and the block date
    /// of the current chain tip as known by the serving node.
    type TipFuture: Future<Item = (T::Id, T::Date), Error = Error>;

    fn tip(&mut self) -> Self::TipFuture;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `stream_blocks_to_tip`.
    type StreamBlocksToTipStream: Stream<Item = T, Error = Error>;

    /// The type of asynchronous futures returned by method `stream_blocks_to_tip`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type StreamBlocksToTipFuture: Future<Item = Self::StreamBlocksToTipStream, Error = Error>;

    fn stream_blocks_to_tip(&mut self, from: &[T::Id]) -> Self::StreamBlocksToTipFuture;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `get_blocks`.
    type GetBlocksStream: Stream<Item = T, Error = Error>;

    /// The type of asynchronous futures returned by method `get_blocks`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetBlocksFuture: Future<Item = Self::GetBlocksStream, Error = Error>;
}

/// Interface for the blockchain node service responsible for
/// providing access to block headers.
pub trait HeaderService<T: HasHeader> {
    /// The type of an asynchronous stream that provides block headers in
    /// response to method `get_headers`.
    type GetHeadersStream: Stream<Item = T::Header, Error = Error>;

    /// The type of asynchronous futures returned by method `get_headers`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetHeadersFuture: Future<Item = Self::GetHeadersStream, Error = Error>;
}
