//! Blockchain content service abstraction.

use super::{request_stream, P2pService};
use crate::error::Error;

use chain_core::property::{Fragment, FragmentId};

use futures::prelude::*;

/// Interface for the blockchain node service implementation responsible for
/// validating and accepting transactions and other block contents, known
/// together as fragments.
pub trait FragmentService: P2pService {
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

    /// The type of a bidirectional subscription object that is used as:
    ///
    /// - a stream for outbound fragments;
    /// - a sink for inbound fragments.
    type FragmentSubscription: Stream<Item = Self::Fragment, Error = Error>
        + Sink<SinkItem = Self::Fragment, SinkError = Error>
        + request_stream::MapResponse<Response = ()>
        + Send
        + 'static;

    /// The type of asynchronous futures returned by method `content_subscription`.
    ///
    /// The future, when successful, resolves to a subscription object
    /// for bidirectional streaming.
    type FragmentSubscriptionFuture: Future<Item = Self::FragmentSubscription, Error = Error>
        + Send
        + 'static;

    /// Get all transactions by their id.
    fn get_fragments(&mut self, ids: &[Self::FragmentId]) -> Self::GetFragmentsFuture;

    /// Establishes a bidirectional subscription for exchanging new block
    /// fragments.
    ///
    /// The network protocol implementation passes the node identifier of
    /// the sender node.
    ///
    /// The implementation of the method returns a future, resolving
    /// to an object that serves as both an asynchronous stream for
    /// outbound fragment messages, and as an asynchrounous sink for inbound
    /// fragment messages.
    fn fragment_subscription(
        &mut self,
        subscriber: Self::NodeId,
    ) -> Self::FragmentSubscriptionFuture;
}
