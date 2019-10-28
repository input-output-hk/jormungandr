///! Gossip service abstraction.
use super::{request_stream, P2pService};
use crate::{
    error::Error,
    gossip::{Gossip, Node},
};

use futures::prelude::*;

/// Interface for the node gossip service implementation
/// in the peer-to-peer network.
pub trait GossipService: P2pService {
    /// Gossip message describing a network node.
    type Node: Node<Id = Self::NodeId>;

    /// The type of a bidirectional subscription object that is used as:
    ///
    /// - a stream for outbound gossip;
    /// - a sink for inbound gossip.
    type GossipSubscription: Stream<Item = Gossip<Self::Node>, Error = Error>
        + Sink<SinkItem = Gossip<Self::Node>, SinkError = Error>
        + request_stream::MapResponse<Response = ()>
        + Send
        + 'static;

    /// The type of asynchronous futures returned by method `gossip_subscription`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GossipSubscriptionFuture: Future<Item = Self::GossipSubscription, Error = Error>
        + Send
        + 'static;

    /// Establishes a bidirectional subscription for node gossip messages.
    ///
    /// The network protocol implementation passes the node identifier of
    /// the sender node.
    ///
    /// The implementation of the method returns a future, resolving
    /// to an object that serves as both an asynchronous stream for
    /// outbound gossip messages, and as an asynchrounous sink for inbound
    /// gossip messages.
    fn gossip_subscription(&mut self, subscriber: Self::NodeId) -> Self::GossipSubscriptionFuture;
}
