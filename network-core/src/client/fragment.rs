use super::p2p::P2pService;
use crate::error::Error;
use chain_core::property::Fragment;

use futures::prelude::*;

/// Interface for the blockchain node service responsible for
/// providing access to block content known as fragments.
pub trait FragmentService: P2pService {
    /// The data type to represent fragments constituting a block.
    type Fragment: Fragment;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `get_blocks`.
    type GetFragmentsStream: Stream<Item = Self::Fragment, Error = Error>;

    /// The type of asynchronous futures returned by method `get_fragments`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetFragmentsFuture: Future<Item = Self::GetFragmentsStream, Error = Error>;

    /// Retrieves the identified blocks in an asynchronous stream.
    fn get_fragments(
        &mut self,
        ids: &[<Self::Fragment as Fragment>::Id],
    ) -> Self::GetFragmentsFuture;

    /// The type of asynchronous futures returned by method `content_subscription`.
    ///
    /// The future resolves to a stream of fragments sent by the remote node
    /// and the identifier of the node in the network.
    type FragmentSubscriptionFuture: Future<
        Item = (Self::FragmentSubscription, Self::NodeId),
        Error = Error,
    >;

    /// The type of an asynchronous stream that provides notifications
    /// of fragments created or accepted by the remote node.
    type FragmentSubscription: Stream<Item = Self::Fragment, Error = Error>;

    /// Establishes a bidirectional stream of notifications for fragments
    /// created or accepted by either of the peers.
    ///
    /// The client can use the stream that the returned future resolves to
    /// as a long-lived subscription handle.
    fn fragment_subscription<S>(&mut self, outbound: S) -> Self::FragmentSubscriptionFuture
    where
        S: Stream<Item = Self::Fragment> + Send + 'static;
}
