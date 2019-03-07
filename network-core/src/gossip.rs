use chain_core::property::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct NodeId([u8; 16]);

pub enum NodeIdError {
    InvalidSize(usize),
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
