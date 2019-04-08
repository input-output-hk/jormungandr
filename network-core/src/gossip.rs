use chain_core::property::{Deserialize, Serialize};

use std::{
    iter::{DoubleEndedIterator, FromIterator, FusedIterator},
    net::SocketAddr,
    vec,
};

/// Marker trait for the type representing a node ID.
pub trait NodeId: Clone + Serialize + Deserialize {}

/// Abstract trait for data types representing gossip about network nodes.
pub trait Node: Serialize + Deserialize {
    /// Type that represents the node identifier in the gossip message.
    type Id: NodeId;

    /// Returns the node identifier.
    fn id(&self) -> Self::Id;

    /// Returns the TCP socket address, if available for this node.
    fn address(&self) -> Option<SocketAddr>;
}

#[derive(Clone, Debug)]
pub struct Gossip<T: Node> {
    nodes: Vec<T>,
}

impl<T: Node> Gossip<T> {
    pub fn from_nodes<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Gossip {
            nodes: Vec::from_iter(iter),
        }
    }

    pub fn nodes(&self) -> &[T] {
        &self.nodes
    }

    pub fn into_nodes(self) -> IntoNodes<T> {
        IntoNodes {
            inner: self.nodes.into_iter(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct IntoNodes<T> {
    inner: vec::IntoIter<T>,
}

impl<T> Iterator for IntoNodes<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoNodes<T> {
    fn next_back(&mut self) -> Option<T> {
        self.inner.next_back()
    }
}

impl<T> ExactSizeIterator for IntoNodes<T> {}
impl<T> FusedIterator for IntoNodes<T> {}
