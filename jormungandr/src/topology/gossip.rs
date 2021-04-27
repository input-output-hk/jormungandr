use super::{limits, NodeId};
use crate::network::p2p::Address;

use chain_core::property;
use std::net::{IpAddr, Ipv4Addr};
use thiserror::Error;

use bincode::Options;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Gossip(poldercast::Gossip);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gossips(pub Vec<Gossip>);

impl Gossip {
    #[inline]
    pub fn address(&self) -> Address {
        self.0.address()
    }

    #[inline]
    pub fn id(&self) -> NodeId {
        NodeId(self.0.id())
    }

    pub fn has_valid_address(&self) -> bool {
        let addr = self.address();

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

        let addr = self.address();

        fn is_ipv4_global(ip: Ipv4Addr) -> bool {
            if ip.is_private() {
                return false;
            }
            if ip.is_loopback() {
                return false;
            }
            if ip.is_link_local() {
                return false;
            }
            true
        }

        match addr.ip() {
            IpAddr::V4(ip) => is_ipv4_global(ip),
            IpAddr::V6(ip) => {
                if ip.is_loopback() {
                    return false;
                }
                // Check using same methods by trying to cast address to ipv4
                // FIXME: use Ipv6 tests when Ipv6Addr convenience methods get stabilized:
                // https://github.com/rust-lang/rust/issues/27709
                if let Some(ipv4) = ip.to_ipv4() {
                    if !is_ipv4_global(ipv4) {
                        return false;
                    }
                }
                true
            }
        }
    }
}

impl From<Gossip> for poldercast::Gossip {
    fn from(gossip: Gossip) -> Self {
        gossip.0
    }
}

impl From<poldercast::Gossip> for Gossip {
    fn from(profile: poldercast::Gossip) -> Self {
        Gossip(profile)
    }
}

impl From<Vec<poldercast::Gossip>> for Gossips {
    fn from(gossips: Vec<poldercast::Gossip>) -> Gossips {
        Gossips(gossips.into_iter().map(Gossip::from).collect())
    }
}

impl From<Gossips> for Vec<poldercast::Gossip> {
    fn from(gossips: Gossips) -> Vec<poldercast::Gossip> {
        gossips.0.into_iter().map(|gossip| gossip.0).collect()
    }
}

impl From<Vec<Gossip>> for Gossips {
    fn from(gossips: Vec<Gossip>) -> Self {
        Gossips(gossips)
    }
}

#[derive(Debug, Error)]
pub enum GossipError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Bincode(#[from] bincode::Error),
    #[error(transparent)]
    InvalidGossip(#[from] poldercast::GossipError),
}

impl property::Serialize for Gossip {
    type Error = GossipError;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        let config = bincode::options();
        config.with_limit(limits::MAX_GOSSIP_SIZE);

        Ok(config.serialize_into(writer, &self.0.as_ref())?)
    }
}

impl property::Deserialize for Gossip {
    type Error = GossipError;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let config = bincode::options();
        config.with_limit(limits::MAX_GOSSIP_SIZE);

        config
            .deserialize_from::<R, Vec<u8>>(reader)
            .map_err(GossipError::from)
            .and_then(|slice| {
                Ok(Gossip(
                    poldercast::GossipSlice::try_from_slice(&slice)?.to_owned(),
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

    // Build a gossip with a random key, just for testing addresses
    fn build_gossip(addr: SocketAddr) -> Gossip {
        Gossip::from(poldercast::Gossip::new(
            addr,
            &keynesis::key::ed25519::SecretKey::new(rand_chacha::ChaChaRng::from_seed([0u8; 32])),
            poldercast::Subscriptions::new().as_slice(),
        ))
    }

    #[test]
    fn gossip_global_ipv4_private() {
        let ip = Ipv4Addr::new(10, 0, 0, 1);
        let addr = SocketAddr::V4(SocketAddrV4::new(ip, 1234));
        assert!(!build_gossip(addr).is_global());
    }

    #[test]
    fn gossip_global_ipv6_private() {
        let ip = Ipv4Addr::new(0, 0, 0, 0);
        let ipv6 = ip.to_ipv6_compatible();
        let addr = SocketAddr::V6(SocketAddrV6::new(ipv6, 1234, 0, 0));
        assert!(!build_gossip(addr).is_global());
    }

    #[test]
    fn gossip_global_ipv4_loopback() {
        let ip = Ipv4Addr::new(127, 255, 255, 255);
        // Address should not be private but be loopback
        assert!(!ip.is_private());
        let addr = SocketAddr::V4(SocketAddrV4::new(ip, 1234));
        assert!(!build_gossip(addr).is_global());
    }

    #[test]
    fn gossip_global_ipv6_loopback() {
        let ip = Ipv4Addr::new(127, 255, 255, 255);
        // Address should not be private but be loopback
        assert!(!ip.is_private());
        let ipv6 = ip.to_ipv6_compatible();
        let addr = SocketAddr::V6(SocketAddrV6::new(ipv6, 1234, 0, 0));
        assert!(!build_gossip(addr).is_global());
    }

    #[test]
    fn gossip_global_ipv4_link_local() {
        let ip = Ipv4Addr::new(169, 254, 10, 65);
        // Address should not be private nor loopback
        assert!(!ip.is_private());
        assert!(!ip.is_loopback());
        let addr = SocketAddr::V4(SocketAddrV4::new(ip, 1234));
        assert!(!build_gossip(addr).is_global());
    }

    #[test]
    fn gossip_global_ipv6_link_local() {
        let ip = Ipv4Addr::new(169, 254, 10, 65);
        // Address should not be private not loopback
        assert!(!ip.is_private());
        assert!(!ip.is_loopback());
        let ipv6 = ip.to_ipv6_compatible();
        let addr = SocketAddr::V6(SocketAddrV6::new(ipv6, 1234, 0, 0));
        assert!(!build_gossip(addr).is_global());
    }
}
