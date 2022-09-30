use super::{limits, NodeId};
use crate::network::p2p::Address;
use chain_core::{packer::Codec, property};
use std::net::{IpAddr, Ipv4Addr};
use thiserror::Error;

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
    ReadError(#[from] property::ReadError),
    #[error(transparent)]
    InvalidGossip(#[from] poldercast::GossipError),
}

// After updating Gossip serde format also need to update it in peers() function in 'testing/jormungandr-automation/src/jormungandr/grpc/server/mod.rs'
impl property::Serialize for Gossip {
    fn serialize<W: std::io::Write>(
        &self,
        codec: &mut Codec<W>,
    ) -> Result<(), property::WriteError> {
        let bytes = self.0.as_ref();
        if bytes.len() > limits::MAX_GOSSIP_SIZE {
            return Err(property::WriteError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Gossip size more than expected",
            )));
        }
        codec.put_be_u16(bytes.len() as u16)?;
        codec.put_bytes(bytes)
    }
}

impl property::Deserialize for Gossip {
    fn deserialize<R: std::io::Read>(codec: &mut Codec<R>) -> Result<Self, property::ReadError> {
        let bytes_size = codec.get_be_u16()? as usize;
        if bytes_size > limits::MAX_GOSSIP_SIZE {
            return Err(property::ReadError::SizeTooBig(
                limits::MAX_GOSSIP_SIZE as usize,
                bytes_size,
            ));
        }
        let bytes = codec.get_bytes(bytes_size as usize)?;
        Ok(Gossip(
            poldercast::GossipSlice::try_from_slice(bytes.as_slice())
                .map_err(|e| property::ReadError::InvalidData(e.to_string()))?
                .to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chain_impl_mockchain::testing::serialization::serialization_bijection;
    use quickcheck::{quickcheck, Arbitrary, TestResult};
    use rand::SeedableRng;
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

    impl Arbitrary for Gossip {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let ip = Ipv4Addr::new(
                u8::arbitrary(g),
                u8::arbitrary(g),
                u8::arbitrary(g),
                u8::arbitrary(g),
            );
            let addr = SocketAddr::V4(SocketAddrV4::new(ip, u16::arbitrary(g)));
            build_gossip(addr)
        }
    }

    // Build a gossip with a random key, just for testing addresses
    fn build_gossip(addr: SocketAddr) -> Gossip {
        Gossip::from(poldercast::Gossip::new(
            addr,
            &keynesis::key::ed25519::SecretKey::new(rand_chacha::ChaChaRng::from_seed([0u8; 32])),
            poldercast::Subscriptions::new().as_slice(),
        ))
    }

    quickcheck! {
        fn gossip_serialization_bijection(b: Gossip) -> TestResult {
            serialization_bijection(b)
        }
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
