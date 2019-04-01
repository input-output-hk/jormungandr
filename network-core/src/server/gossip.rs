///! Gossip service abstraction.
use crate::{
    error::Error,
    gossip::{Gossip, Node, NodeId},
};

use futures::prelude::*;

/// Intreface for the node discovery service implementation
/// in the p2p network.
pub trait GossipService {
    /// Network node identifier.
    type NodeId: NodeId;

    /// Gossip message describing a network node.
    type Node: Node<Id = Self::NodeId>;

    /// The type of an asynchronous stream that retrieves node gossip
    /// messages from a peer.
    type GossipSubscription: Stream<Item = Gossip<Self::Node>, Error = Error>;

    /// The type of asynchronous futures returned by method `gossip_subscription`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GossipSubscriptionFuture: Future<Item = Self::GossipSubscription, Error = Error>;

    // Establishes a bidirectional subscription for node gossip messages,
    // taking an asynchronous stream that provides the inbound announcements.
    //
    // Returns a future that resolves to an asynchronous subscription stream
    // that receives node gossip messages sent by the peer.
    fn gossip_subscription<In>(&mut self, inbound: In) -> Self::GossipSubscriptionFuture
    where
        In: Stream<Item = Gossip<Self::Node>, Error = Error> + Send + 'static;
}
