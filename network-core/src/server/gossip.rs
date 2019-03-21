///! Gossip service abstraction.
use crate::{
    error::Error,
    gossip::{self, Gossip},
};

use futures::prelude::*;

/// Intreface for the node discovery service implementation
/// in the p2p network.
pub trait GossipService {
    /// Gossip message content.
    type Message: Gossip;

    type MessageFuture: Future<Item = (gossip::NodeId, Self::Message), Error = Error>;

    /// Record and process gossip event.
    fn record_gossip(
        &mut self,
        node_id: gossip::NodeId,
        gossip: &Self::Message,
    ) -> Self::MessageFuture;
}
