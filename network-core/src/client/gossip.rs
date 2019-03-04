use super::Error;

use super::super::gossip::{Gossip, NodeId};

use futures::prelude::*;

pub trait GossipService {
    type Gossip: Gossip;

    /// Future that represent an asynchronous gossip request.
    type GossipFuture: Future<Item = (NodeId, Self::Gossip), Error = Error>;

    /// Request to announce our own gossip and bring back result.
    fn gossip(&mut self, node_id: &NodeId, gossip: &Self::Gossip) -> Self::GossipFuture;
}
