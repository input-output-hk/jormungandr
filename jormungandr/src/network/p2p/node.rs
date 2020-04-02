use crate::network::p2p::Id;
use bincode;
use chain_core::property;
use std::net::SocketAddr;

pub struct Node {
    info: poldercast::NodeInfo,
}

impl Node {
    pub fn new(info: poldercast::NodeInfo) -> Self {
        Self { info }
    }

    #[inline]
    pub fn id(&self) -> Id {
        (*self.info.id()).into()
    }

    #[inline]
    pub fn address(&self) -> Option<SocketAddr> {
        self.info.address().map(|addr| addr.to_socketaddr())
    }
}

impl From<Node> for poldercast::NodeInfo {
    fn from(node: Node) -> Self {
        node.info
    }
}

#[deprecated]
impl property::Serialize for Node {
    type Error = bincode::Error;

    fn serialize<W: std::io::Write>(&self, _writer: W) -> Result<(), Self::Error> {
        unreachable!("This part of the code should never been called or use")
    }
}

#[deprecated]
impl property::Deserialize for Node {
    type Error = bincode::Error;

    fn deserialize<R: std::io::BufRead>(_reader: R) -> Result<Self, Self::Error> {
        unreachable!("This part of the code should never been called or use")
    }
}
