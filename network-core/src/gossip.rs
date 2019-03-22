use chain_core::property::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Abstract trait for data types representing gossip about network nodes.
pub trait NodeGossip {
    /// Type that represents the node identifier in the gossip message.
    type NodeId: Serialize + Deserialize;

    /// Constructs a new instance from an id and a socket address.
    fn new(id: Self::NodeId, addr: SocketAddr) -> Self;

    /// Returns the node identifier.
    fn id(&self) -> Self::NodeId;

    /// Returns the TCP socket address.
    fn addr(&self) -> SocketAddr;
}
