use chain_core::property::{Deserialize, Serialize};

use std::{iter::FromIterator, net::SocketAddr};

/// Marker trait for the type representing a node ID.
pub trait NodeId: Serialize + Deserialize {}

/// Abstract trait for data types representing gossip about network nodes.
pub trait Node: Serialize + Deserialize {
    /// Type that represents the node identifier in the gossip message.
    type Id: NodeId;

    /// Returns the node identifier.
    fn id(&self) -> Self::Id;

    /// Returns the TCP socket address, if available for this node.
    fn address(&self) -> Option<SocketAddr>;
}

pub struct Gossip<T: Node> {
    sender: T::Id,
    nodes: Vec<T>,
}

impl<T: Node> Gossip<T> {
    pub fn from_nodes<I>(sender: T::Id, iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Gossip {
            sender,
            nodes: Vec::from_iter(iter.into_iter()),
        }
    }

    pub fn sender(&self) -> &T::Id {
        &self.sender
    }

    pub fn nodes(&self) -> &[T] {
        &self.nodes
    }
}
