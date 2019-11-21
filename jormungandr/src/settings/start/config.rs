use crate::{
    network::p2p::{topic, Id, PolicyConfig},
    settings::logging::{LogFormat, LogOutput},
    settings::LOG_FILTER_LEVEL_POSSIBLE_VALUES,
};
use jormungandr_lib::{interfaces::Mempool, time::Duration};
use poldercast;
use serde::{de::Error as _, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use slog::FilterLevel;
use std::{collections::BTreeMap, fmt, net::SocketAddr, path::PathBuf};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub secret_files: Vec<PathBuf>,
    pub storage: Option<PathBuf>,
    pub log: Option<ConfigLogSettings>,

    /// setting of the mempool, fragment logs and related data
    #[serde(default)]
    pub mempool: Mempool,

    #[serde(default)]
    pub leadership: Leadership,

    pub rest: Option<Rest>,

    #[serde(default)]
    pub p2p: P2pConfig,

    pub explorer: Option<Explorer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigLogSettingsEntry {
    #[serde(with = "filter_level_opt_serde")]
    pub level: Option<FilterLevel>,
    pub format: Option<LogFormat>,
    pub output: Option<LogOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigLogSettings(pub Vec<ConfigLogSettingsEntry>);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rest {
    pub listen: SocketAddr,
    pub pkcs12: Option<PathBuf>,
    /// Enables CORS if provided
    pub cors: Option<Cors>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Cors {
    /// If none provided, echos request origin
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    /// If none provided, CORS responses won't be cached
    pub max_age_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct P2pConfig {
    /// The public address to which other peers may connect to
    pub public_address: Option<Address>,

    /// The socket address to listen on, if different from the public address.
    /// The format is "{ip_address}:{port}".
    /// The IP address can be specified as 0.0.0.0 or :: to listen on
    /// all network interfaces.
    pub listen_address: Option<Address>,

    pub public_id: Option<Id>,

    /// the rendezvous points for the peer to connect to in order to initiate
    /// the p2p discovery from.
    pub trusted_peers: Option<Vec<TrustedPeer>>,
    /// the topic subscriptions
    ///
    /// When connecting to different nodes we will expose these too in order to
    /// help the different modules of the P2P topology engine to determine the
    /// best possible neighborhood.
    pub topics_of_interest: Option<BTreeMap<Topic, InterestLevel>>,

    /// Limit on the number of simultaneous connections.
    /// If not specified, an internal default limit is used.
    pub max_connections: Option<usize>,

    /// Whether to allow non-public IP addresses on the network.
    /// The default is to not allow advertising non-public IP addresses.
    #[serde(default)]
    pub allow_private_addresses: bool,

    /// setting for the policy
    #[serde(default)]
    pub policy: PolicyConfig,

    /// set the maximum number of unreachable nodes to contact at a time for every
    /// new notification. The default value is 20.
    ///
    /// Every time a new propagation event is triggered, the node will select
    /// randomly a certain amount of unreachable nodes to connect to in addition
    /// to the one selected by other p2p topology layer.
    #[serde(default)]
    pub max_unreachable_nodes_to_connect_per_event: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedPeer {
    pub address: TrustedAddress,
    pub id: Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Leadership {
    /// LeadershipLog time to live, it is for information purposes, we log all the Leadership
    /// event logs in a cache. The log will be discarded at the end of the ttl.
    pub log_ttl: Duration,
    /// interval between 2 garbage collection check logs
    pub garbage_collection_interval: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address(pub poldercast::Address);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedAddress(pub multiaddr::Multiaddr);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Topic(pub poldercast::Topic);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterestLevel(pub poldercast::InterestLevel);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Explorer {
    pub enabled: bool,
}

pub fn default_interests() -> BTreeMap<Topic, InterestLevel> {
    use std::iter::FromIterator as _;

    BTreeMap::from_iter(vec![
        (
            Topic(topic::MESSAGES),
            InterestLevel(poldercast::InterestLevel::Low),
        ),
        (
            Topic(topic::BLOCKS),
            InterestLevel(poldercast::InterestLevel::Normal),
        ),
    ])
}

impl Default for P2pConfig {
    fn default() -> Self {
        P2pConfig {
            public_address: None,
            listen_address: None,
            public_id: None,
            trusted_peers: None,
            topics_of_interest: None,
            max_connections: None,
            allow_private_addresses: false,
            policy: PolicyConfig::default(),
            max_unreachable_nodes_to_connect_per_event: None,
        }
    }
}

impl Default for Leadership {
    fn default() -> Self {
        Leadership {
            log_ttl: Duration::new(3600, 0),
            garbage_collection_interval: Duration::new(3600 / 4, 0),
        }
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

impl Address {
    pub fn to_socketaddr(&self) -> Option<SocketAddr> {
        self.0.to_socketaddr()
    }
}

custom_error! {pub AddressError
    DnsLookupError { source: std::io::Error } = "failed to resolve DNS name {source}",
    NoPortSpecified = "no TCP port specified",
    NoAppropriateDNSFound = "the address was resolved, but it doesn't provide IPv4 or IPv6 addresses",
    UnsupportedProtocol = "the provided protocol is unsupported, please use one of ip4/ip6/dns4/dns6",
}

impl TrustedAddress {
    pub fn to_addresses(&self) -> Result<Vec<Address>, AddressError> {
        use multiaddr::AddrComponent;
        use std::{iter::FromIterator, net::ToSocketAddrs};

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

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for TrustedAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self.0))
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

impl Serialize for Topic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if self.0 == topic::MESSAGES.into() {
            serializer.serialize_str("messages")
        } else if self.0 == topic::BLOCKS.into() {
            serializer.serialize_str("blocks")
        } else {
            Err(S::Error::custom("invalid state... should not happen"))
        }
    }
}
impl Serialize for InterestLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            poldercast::InterestLevel::Low => serializer.serialize_str("low"),
            poldercast::InterestLevel::Normal => serializer.serialize_str("normal"),
            poldercast::InterestLevel::High => serializer.serialize_str("high"),
        }
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AddressVisitor;
        impl<'de> Visitor<'de> for AddressVisitor {
            type Value = Address;

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
                    Ok(addr) => Ok(Address(addr)),
                }
            }
        }
        deserializer.deserialize_str(AddressVisitor)
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

impl<'de> Deserialize<'de> for Topic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TopicVisitor;
        impl<'de> Visitor<'de> for TopicVisitor {
            type Value = Topic;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "Topic: messages or blocks")
            }

            fn visit_str<'a, E>(self, v: &'a str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use serde::de::Unexpected;

                match v {
                    "messages" => Ok(Topic(topic::MESSAGES.into())),
                    "blocks" => Ok(Topic(topic::BLOCKS.into())),
                    err => Err(E::invalid_value(Unexpected::Str(err), &self)),
                }
            }
        }
        deserializer.deserialize_str(TopicVisitor)
    }
}

impl<'de> Deserialize<'de> for InterestLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct InterestLevelVisitor;
        impl<'de> Visitor<'de> for InterestLevelVisitor {
            type Value = InterestLevel;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "Interest Level: low, normal or high")
            }

            fn visit_str<'a, E>(self, v: &'a str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use serde::de::Unexpected;

                match v {
                    "low" => Ok(InterestLevel(poldercast::InterestLevel::Low)),
                    "normal" => Ok(InterestLevel(poldercast::InterestLevel::Normal)),
                    "high" => Ok(InterestLevel(poldercast::InterestLevel::High)),
                    err => Err(E::invalid_value(Unexpected::Str(err), &self)),
                }
            }
        }
        deserializer.deserialize_str(InterestLevelVisitor)
    }
}

mod filter_level_opt_serde {
    use super::*;

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<FilterLevel>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(|variant| {
                variant.parse().map_err(|_| {
                    D::Error::unknown_variant(&variant, &**LOG_FILTER_LEVEL_POSSIBLE_VALUES)
                })
            })
            .transpose()
    }

    pub fn serialize<S: Serializer>(
        data: &Option<FilterLevel>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        data.map(|level| level.as_str()).serialize(serializer)
    }
}
