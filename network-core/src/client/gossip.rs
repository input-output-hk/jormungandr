use super::p2p::P2pService;
use crate::{
    error::Error,
    gossip::{Gossip, Node},
};

use futures::prelude::*;

pub trait GossipService: P2pService {
    type Node: Node<Id = Self::NodeId>;

    /// The type of an asynchronous stream that provides node gossip messages
    /// sent by the peer.
    type GossipSubscription: Stream<Item = Gossip<Self::Node>, Error = Error>;

    /// The type of asynchronous futures returned by method `gossip_subscription`.
    ///
    /// The future resolves to a stream of gossip messages sent by the remote node
    /// and the identifier of the node in the network.
    type GossipSubscriptionFuture: Future<
        Item = (Self::GossipSubscription, Self::NodeId),
        Error = Error,
    >;

    /// Establishes a bidirectional stream of notifications for gossip
    /// messages.
    ///
    /// The client can use the stream that the returned future resolves to
    /// as a long-lived subscription handle.
    fn gossip_subscription<S>(&mut self, outbound: S) -> Self::GossipSubscriptionFuture
    where
        S: Stream<Item = Gossip<Self::Node>> + Send + 'static;
}
