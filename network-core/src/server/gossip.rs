///! Gossip service abstraction.
use crate::{error::Error, gossip::NodeGossip};

use futures::prelude::*;

/// Intreface for the node discovery service implementation
/// in the p2p network.
pub trait GossipService {
    /// Gossip message content.
    type NodeGossip: NodeGossip;

    /// The type of an asynchronous stream that retrieves node gossip
    /// messages from a peer.
    type GossipSubscription: Stream<Item = Self::NodeGossip, Error = Error>;

    /// The type of asynchronous futures returned by method `gossip_subscription`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GossipSubscriptionFuture: Future<Item = Self::GossipSubscription, Error = Error>;

    // Establishes a bidirectional subscription for node gossip messages,
    // taking an asynchronous stream that provides the outbound announcements.
    //
    // Returns a future that resolves to an asynchronous subscription stream
    // that receives node gossip messages sent by the peer.
    fn gossip_subscription<Out>(&mut self, outbound: Out) -> Self::GossipSubscriptionFuture
    where
        Out: Stream<Item = Self::NodeGossip, Error = Error>;
}
