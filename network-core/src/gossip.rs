use chain_core::property::{Deserialize, Serialize};
use std::{error, fmt};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct NodeId([u8; 16]);

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

impl NodeId {
    pub fn from_slice(slice: &[u8]) -> Result<Self, NodeIdError> {
        if slice.len() != 16 {
            return Err(NodeIdError::InvalidSize(slice.len()));
        }
        let mut buf = [0; 16];
        buf[0..16].clone_from_slice(slice);
        Ok(NodeId(buf))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0[..].into()
    }
}

pub trait Gossip: Serialize + Deserialize {
    /// Type that represents NodeId in the gossip message.
    type NodeId: Sized;
    /// Information about the node that is kept in the gossip message.
    type Node;

    fn from_nodes<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Self::Node>;
}
