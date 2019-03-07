use super::Error;

use super::super::gossip::{self, Gossip};

use futures::prelude::*;

pub trait GossipService {
    type Gossip: Gossip;

    /// Future that represent an asynchronous gossip request.
    type GossipFuture: Future<Item = (gossip::NodeId, Self::Gossip), Error = Error>;

    /// Request to announce our own gossip and bring back result.
    fn gossip(&mut self, node_id: &gossip::NodeId, gossip: &Self::Gossip) -> Self::GossipFuture;
}
