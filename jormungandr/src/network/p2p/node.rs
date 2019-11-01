use crate::network::p2p::Id;
use bincode;
use chain_core::property;
use network_core::gossip;
use std::net::SocketAddr;

pub struct Node {
    info: poldercast::NodeInfo,
}

impl Node {
    pub fn new(info: poldercast::NodeInfo) -> Self {
        Self { info }
    }
}

impl gossip::Node for Node {
    type Id = Id;

    #[inline]
    fn id(&self) -> Self::Id {
        (*self.info.id()).into()
    }

    #[inline]
    fn address(&self) -> Option<SocketAddr> {
        if let Some(address) = self.info.address() {
            address.to_socketaddr()
        } else {
            None
        }
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
