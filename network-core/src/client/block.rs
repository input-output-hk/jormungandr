use super::Error;

use chain_core::property::{Block, HasHeader};

use futures::prelude::*;

/// Interface for the blockchain node service responsible for
/// providing access to blocks.
pub trait BlockService {
    /// The type of blockchain block served by this service.
    type Block: Block + HasHeader;

    /// The type of asynchronous futures returned by method `tip`.
    ///
    /// The future resolves to the block identifier and the block date
    /// of the current chain tip as known by the serving node.
    type TipFuture: Future<Item = <Self::Block as HasHeader>::Header, Error = Error>;

    fn tip(&mut self) -> Self::TipFuture;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `pull_blocks_to_tip`.
    type PullBlocksToTipStream: Stream<Item = Self::Block, Error = Error>;

    /// The type of asynchronous futures returned by method `pull_blocks_to_tip`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type PullBlocksToTipFuture: Future<Item = Self::PullBlocksToTipStream, Error = Error>;

    fn pull_blocks_to_tip(
        &mut self,
        from: &[<Self::Block as Block>::Id],
    ) -> Self::PullBlocksToTipFuture;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `get_blocks`.
    type GetBlocksStream: Stream<Item = Self::Block, Error = Error>;

    /// The type of asynchronous futures returned by method `get_blocks`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetBlocksFuture: Future<Item = Self::GetBlocksStream, Error = Error>;

    // The type of an asynchronous stream that provides block headers in
    // response to method `get_headers`.
    //type GetHeadersStream: Stream<Item = <Self::Block as Block>::Header, Error = Error>;

    // The type of asynchronous futures returned by method `get_headers`.
    //
    // The future resolves to a stream that will be used by the protocol
    // implementation to produce a server-streamed response.
    //type GetHeadersFuture: Future<Item = Self::GetHeadersStream, Error = Error>;

    /// The type of asynchronous futures returned by method `subscribe`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a subscription stream.
    type BlockSubscriptionFuture: Future<Item = Self::BlockSubscription, Error = Error>;

    /// The type of an asynchronous stream that provides notifications
    /// of blocks created or accepted by the remote node.
    type BlockSubscription: Stream<Item = <Self::Block as HasHeader>::Header, Error = Error>;

    /// Establishes a bidirectional stream of notifications for blocks
    /// created or accepted by either of the peers.
    ///
    /// The client can use the stream that the returned future resolves to
    /// as a long-lived subscription handle.
    fn subscription<S>(&mut self, outbound: S) -> Self::BlockSubscriptionFuture
    where
        S: Stream<Item = <Self::Block as HasHeader>::Header> + Send + 'static;
}
