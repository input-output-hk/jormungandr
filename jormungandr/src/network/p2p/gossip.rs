use crate::network::p2p::{limits, Id};
use bincode;
use chain_core::property;
use network_core::gossip::{self, Node as _};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Gossip(poldercast::NodeProfile);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gossips(poldercast::Gossips);

impl Gossip {
    pub fn has_valid_address(&self) -> bool {
        let addr = match self.address() {
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

    pub fn is_global(&self) -> bool {
        if !self.has_valid_address() {
            return false;
        }

        let addr = match self.address() {
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
                // FIXME: add more tests when Ipv6Addr convenience methods
                // get stabilized:
                // https://github.com/rust-lang/rust/issues/27709
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

impl From<Gossips> for network_core::gossip::Gossip<Gossip> {
    fn from(gossips: Gossips) -> Self {
        network_core::gossip::Gossip::from_nodes(gossips.0.into_iter().map(Gossip))
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

impl gossip::Node for Gossip {
    type Id = Id;

    #[inline]
    fn id(&self) -> Self::Id {
        (*self.0.id()).into()
    }

    #[inline]
    fn address(&self) -> Option<SocketAddr> {
        if let Some(address) = self.0.address() {
            address.to_socketaddr()
        } else {
            None
        }
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
