///! Gossip service abstraction.
use super::P2pService;
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

    /// The type of an asynchronous stream that retrieves node gossip
    /// messages from a peer.
    type GossipSubscription: Stream<Item = Gossip<Self::Node>, Error = Error>;

    /// The type of asynchronous futures returned by method `gossip_subscription`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GossipSubscriptionFuture: Future<Item = Self::GossipSubscription, Error = Error>;

    /// Establishes a bidirectional subscription for node gossip messages.
    ///
    /// The network protocol implementation passes the node identifier of
    /// the sender and an asynchronous stream that will provide the inbound
    /// announcements.
    ///
    /// Returns a future resolving to an asynchronous stream
    /// that will be used by this node to send node gossip messages.
    fn gossip_subscription<In>(
        &mut self,
        subscriber: Self::NodeId,
        inbound: In,
    ) -> Self::GossipSubscriptionFuture
    where
        In: Stream<Item = Gossip<Self::Node>, Error = Error> + Send + 'static;
}
