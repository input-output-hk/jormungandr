use crate::{network::p2p::Id, settings::start::config::Address};
use multiaddr::AddrComponent;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt,
    iter::FromIterator,
    net::{SocketAddr, ToSocketAddrs},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedPeer {
    pub address: TrustedAddress,
    pub id: Id,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedAddress(pub multiaddr::Multiaddr);

custom_error! {pub AddressError
    DnsLookupError { source: std::io::Error } = "failed to resolve DNS name {source}",
    NoPortSpecified = "no TCP port specified",
    NoAppropriateDNSFound = "the address was resolved, but it doesn't provide IPv4 or IPv6 addresses",
    UnsupportedProtocol = "the provided protocol is unsupported, please use one of ip4/ip6/dns4/dns6",
}

impl TrustedAddress {
    pub fn to_addresses(&self) -> Result<Vec<Address>, AddressError> {
        let mut components = self.0.iter();
        let protocol = components.next();

        if let Some(AddrComponent::IP4(_)) | Some(AddrComponent::IP6(_)) = protocol {
            return Ok(vec![Address(
                poldercast::Address::new(self.0.clone()).unwrap(),
            )]);
        }

        let port = match components.next() {
            Some(AddrComponent::TCP(port)) => port,
            _ => return Err(AddressError::NoPortSpecified),
        };

        let addresses: Vec<AddrComponent> = match protocol {
            Some(AddrComponent::DNS4(fqdn)) => format!("{}:{}", fqdn, port)
                .to_socket_addrs()
                .map_err(|e| AddressError::DnsLookupError { source: e })?
                .into_iter()
                .filter_map(|r| match r {
                    SocketAddr::V4(addr) => Some(AddrComponent::IP4(*addr.ip())),
                    _ => None,
                })
                .collect(),
            Some(AddrComponent::DNS6(fqdn)) => format!("{}:{}", fqdn, port)
                .to_socket_addrs()
                .map_err(|e| AddressError::DnsLookupError { source: e })?
                .into_iter()
                .filter_map(|r| match r {
                    SocketAddr::V6(addr) => Some(AddrComponent::IP6(*addr.ip())),
                    _ => None,
                })
                .collect(),
            _ => return Err(AddressError::UnsupportedProtocol),
        };

        if addresses.is_empty() {
            return Err(AddressError::NoAppropriateDNSFound);
        }

        let addresses = addresses
            .into_iter()
            .map(|addr| {
                let new_components = vec![addr, AddrComponent::TCP(port)];
                let new_multiaddr = multiaddr::Multiaddr::from_iter(new_components.into_iter());
                Address(poldercast::Address::new(new_multiaddr).unwrap())
            })
            .collect();

        Ok(addresses)
    }
}

impl std::str::FromStr for TrustedPeer {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('@');

        let address = if let Some(address) = split.next() {
            multiaddr::Multiaddr::from_bytes(address.as_bytes().iter().cloned().collect())
                .map(TrustedAddress)
                .map_err(|e| e.to_string())?
        } else {
            return Err("Missing address component".to_owned());
        };

        let id = if let Some(id) = split.next() {
            id.parse::<Id>().map_err(|e| e.to_string())?
        } else {
            return Err("Missing id component".to_owned());
        };

        Ok(TrustedPeer { address, id })
    }
}

impl Serialize for TrustedAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self.0))
    }
}

impl std::fmt::Display for TrustedAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> Deserialize<'de> for TrustedAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TrustedAddressVisitor;
        impl<'de> Visitor<'de> for TrustedAddressVisitor {
            type Value = TrustedAddress;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "Multiaddr (example: /ip4/192.168.0.1/tcp/443)")
            }

            fn visit_str<'a, E>(self, v: &'a str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use serde::de::Unexpected;
                match v.parse() {
                    Err(_err) => Err(E::invalid_value(Unexpected::Str(v), &self)),
                    Ok(addr) => Ok(TrustedAddress(addr)),
                }
            }
        }
        deserializer.deserialize_str(TrustedAddressVisitor)
    }
}
