use crate::{
    error::Error,
    gossip::{Gossip, Node},
};

use futures::prelude::*;

pub trait GossipService {
    type Node: Node;

    /// The type of an asynchronous stream that provides node gossip messages
    /// sent by the peer.
    type GossipSubscription: Stream<Item = Gossip<Self::Node>, Error = Error>;

    /// The type of asynchronous futures returned by method `gossip_subscription`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a subscription stream.
    type GossipSubscriptionFuture: Future<Item = Self::GossipSubscription, Error = Error>;

    /// Establishes a bidirectional stream of notifications for gossip
    /// messages.
    ///
    /// The client can use the stream that the returned future resolves to
    /// as a long-lived subscription handle.
    fn gossip_subscription<S>(&mut self, outbound: S) -> Self::GossipSubscriptionFuture
    where
        S: Stream<Item = Gossip<Self::Node>> + Send + 'static;
}
