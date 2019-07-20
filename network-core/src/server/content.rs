//! Blockchain content service abstraction.

use super::P2pService;
use crate::error::Error;

use chain_core::property::{Fragment, FragmentId};

use futures::prelude::*;

/// Interface for the blockchain node service implementation responsible for
/// validating and accepting transactions and other block contents, known
/// together as fragments.
pub trait ContentService: P2pService {
    /// The data type to represent fragments constituting a block.
    type Fragment: Fragment;

    /// The fragment identifier type for the blockchain.
    type FragmentId: FragmentId;

    /// The type of an asynchronous stream that provides fragments in
    /// response to `get_fragments`.
    type GetFragmentsStream: Stream<Item = Self::Fragment, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by `get_fragments`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetFragmentsFuture: Future<Item = Self::GetFragmentsStream, Error = Error> + Send + 'static;

    /// The type of an asynchronous stream that provides fragments announced
    /// by the peer via the bidirectional subscription.
    type ContentSubscription: Stream<Item = Self::Fragment, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by method `content_subscription`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type ContentSubscriptionFuture: Future<Item = Self::ContentSubscription, Error = Error>
        + Send
        + 'static;

    /// Get all transactions by their id.
    fn get_fragments(&mut self, ids: &[Self::FragmentId]) -> Self::GetFragmentsFuture;

    /// Establishes a bidirectional subscription for announcing new fragments.
    ///
    /// The network protocol implementation passes the node identifier of
    /// the sender and an asynchronous stream that will provide the inbound
    /// announcements.
    ///
    /// Returns a future resolving to an asynchronous stream
    /// that will be used by this node to send fragment announcements.
    fn content_subscription<In>(
        &mut self,
        subscriber: Self::NodeId,
        inbound: In,
    ) -> Self::ContentSubscriptionFuture
    where
        In: Stream<Item = Self::Fragment, Error = Error> + Send + 'static;
}
