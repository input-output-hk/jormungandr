use crate::network::p2p::{limits, Address};
use bincode;
use chain_core::property;
use chain_network::data as net_data;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

pub use net_data::{Peer, Peers};

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Gossip(poldercast::NodeProfile);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gossips(poldercast::Gossips);

impl Gossip {
    #[inline]
    pub fn address(&self) -> Option<&Address> {
        self.0.address()
    }

    pub fn has_valid_address(&self) -> bool {
        let addr = match self.address().and_then(|addr| addr.to_socketaddr()) {
            None => return false,
            Some(addr) => addr,
        };

        match addr.ip() {
            IpAddr::V4(ip) => {
                if ip.is_unspecified() {
                    return false;
                }
                if ip.is_broadcast() {
                    return false;
                }
                if ip.is_multicast() {
                    return false;
                }
                if ip.is_documentation() {
                    return false;
                }
            }
            IpAddr::V6(ip) => {
                if ip.is_unspecified() {
                    return false;
                }
                if ip.is_multicast() {
                    return false;
                }
            }
        }

        true
    }

    /// Check if the bind address is a global address
    /// Note: This method relies on IPV4 checks even for IPV6 addresses. If the IPV6 address
    /// can not be transformed into a IPV4 one then the private and link_local checks are not performed on it.
    pub fn is_global(&self) -> bool {
        if !self.has_valid_address() {
            return false;
        }

        let addr = match self.address().and_then(|addr| addr.to_socketaddr()) {
            None => return false,
            Some(addr) => addr,
        };

        match addr.ip() {
            IpAddr::V4(ip) => {
                if ip.is_private() {
                    return false;
                }
                if ip.is_loopback() {
                    return false;
                }
                if ip.is_link_local() {
                    return false;
                }
            }
            IpAddr::V6(ip) => {
                if ip.is_loopback() {
                    return false;
                }
                // Check using same methods by trying to cast address to ipv4
                // FIXME: use Ipv6 tests when Ipv6Addr convenience methods get stabilized:
                // https://github.com/rust-lang/rust/issues/27709
                match ip.to_ipv4() {
                    Some(ipv4) => {
                        if ipv4.is_private() {
                            return false;
                        }
                        if ipv4.is_link_local() {
                            return false;
                        }
                    }
                    None => {}
                }
            }
        }

        true
    }
}

impl From<Gossip> for poldercast::NodeProfile {
    fn from(gossip: Gossip) -> Self {
        gossip.0
    }
}

impl From<poldercast::NodeProfile> for Gossip {
    fn from(profile: poldercast::NodeProfile) -> Self {
        Gossip(profile)
    }
}

impl From<Gossips> for net_data::gossip::Gossip {
    fn from(gossips: Gossips) -> Self {
        let nodes = gossips
            .0
            .into_iter()
            .map(|node| {
                let gossip = Gossip(node);
                let mut bytes = Vec::new();
                <Gossip as property::Serialize>::serialize(&gossip, &mut bytes).unwrap();
                net_data::gossip::Node::from_bytes(bytes)
            })
            .collect::<Vec<_>>()
            .into();
        net_data::gossip::Gossip { nodes }
    }
}

impl From<poldercast::Gossips> for Gossips {
    fn from(gossips: poldercast::Gossips) -> Gossips {
        Gossips(gossips)
    }
}

impl From<Gossips> for poldercast::Gossips {
    fn from(gossips: Gossips) -> poldercast::Gossips {
        gossips.0
    }
}

impl From<Vec<Gossip>> for Gossips {
    fn from(gossips: Vec<Gossip>) -> Self {
        let v: Vec<_> = gossips.into_iter().map(|gossip| gossip.0).collect();
        Gossips(poldercast::Gossips::from(v))
    }
}

impl property::Serialize for Gossip {
    type Error = bincode::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        let mut config = bincode::config();
        config.limit(limits::MAX_GOSSIP_SIZE);

        config.serialize_into(writer, &self.0)
    }
}

impl property::Deserialize for Gossip {
    type Error = bincode::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let mut config = bincode::config();
        config.limit(limits::MAX_GOSSIP_SIZE);

        config.deserialize_from(reader).map(Gossip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poldercast::{Address, NodeProfileBuilder};
    use std::net::Ipv4Addr;

    #[test]
    fn gossip_global_ipv4_private() {
        let mut builder: NodeProfileBuilder = NodeProfileBuilder::new();
        let ip = Ipv4Addr::new(10, 0, 0, 1);
        builder.address(Address::new(ip).ok().unwrap());
        let node: Gossip = Gossip::from(builder.build());
        assert!(!node.is_global());
    }

    #[test]
    fn gossip_global_ipv6_private() {
        let mut builder: NodeProfileBuilder = NodeProfileBuilder::new();
        let ip = Ipv4Addr::new(0, 0, 0, 0);
        let ipv6 = ip.to_ipv6_compatible();
        builder.address(Address::new(ipv6).ok().unwrap());
        let node: Gossip = Gossip::from(builder.build());
        assert!(!node.is_global());
    }

    #[test]
    fn gossip_global_ipv4_loopback() {
        let mut builder: NodeProfileBuilder = NodeProfileBuilder::new();
        let ip = Ipv4Addr::new(127, 255, 255, 255);
        // Address should not be private but be loopback
        assert!(!ip.is_private());
        builder.address(Address::new(ip).ok().unwrap());
        let node: Gossip = Gossip::from(builder.build());
        assert!(!node.is_global());
    }

    #[test]
    fn gossip_global_ipv6_loopback() {
        let mut builder: NodeProfileBuilder = NodeProfileBuilder::new();
        let ip = Ipv4Addr::new(127, 255, 255, 255);
        // Address should not be private but be loopback
        assert!(!ip.is_private());
        let ipv6 = ip.to_ipv6_compatible();
        builder.address(Address::new(ipv6).ok().unwrap());
        let node: Gossip = Gossip::from(builder.build());
        assert!(!node.is_global());
    }

    #[test]
    fn gossip_global_ipv4_link_local() {
        let mut builder: NodeProfileBuilder = NodeProfileBuilder::new();
        let ip = Ipv4Addr::new(169, 254, 10, 65);
        // Address should not be private nor loopback
        assert!(!ip.is_private());
        assert!(!ip.is_loopback());
        builder.address(Address::new(ip).ok().unwrap());
        let node: Gossip = Gossip::from(builder.build());
        assert!(!node.is_global());
    }

    #[test]
    fn gossip_global_ipv6_link_local() {
        let mut builder: NodeProfileBuilder = NodeProfileBuilder::new();
        let ip = Ipv4Addr::new(169, 254, 10, 65);
        // Address should not be private not loopback
        assert!(!ip.is_private());
        assert!(!ip.is_loopback());
        let ipv6 = ip.to_ipv6_compatible();
        builder.address(Address::new(ipv6).ok().unwrap());
        let node: Gossip = Gossip::from(builder.build());
        assert!(!node.is_global());
    }
}
