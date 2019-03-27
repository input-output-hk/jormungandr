use chain_core::property::{Deserialize, Serialize};
use std::{error, fmt, net::SocketAddr};

#[derive(Clone, Debug)]
pub enum NodeIdError {
    InvalidSize(usize),
}

impl error::Error for NodeIdError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl fmt::Display for NodeIdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeIdError::InvalidSize(size) => write!(f, "invalid node id size: {}", size),
        }
    }
}

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
