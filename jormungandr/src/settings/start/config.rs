use crate::{
    network::p2p::topology::{NEW_BLOCKS_TOPIC, NEW_MESSAGES_TOPIC},
    settings::logging::{LogFormat, LogOutput},
    settings::LOG_FILTER_LEVEL_POSSIBLE_VALUES,
};
use jormungandr_lib::time::Duration;
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
pub struct ConfigLogSettings {
    #[serde(with = "filter_level_opt_serde")]
    pub level: Option<FilterLevel>,
    pub format: Option<LogFormat>,
    pub output: Option<LogOutput>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct P2pConfig {
    /// The public address to which other peers may connect to
    pub public_address: Option<Address>,

    /// The socket address to listen on, if different from the public address.
    /// The format is "{ip_address}:{port}".
    /// The IP address can be specified as 0.0.0.0 or :: to listen on
    /// all network interfaces.
    pub listen_address: Option<Address>,

    /// the rendezvous points for the peer to connect to in order to initiate
    /// the p2p discovery from.
    pub trusted_peers: Option<Vec<poldercast::Address>>,
    /// the topic subscriptions
    ///
    /// When connecting to different nodes we will expose these too in order to
    /// help the different modules of the P2P topology engine to determine the
    /// best possible neighborhood.
    pub topics_of_interest: Option<BTreeMap<Topic, InterestLevel>>,

    /// Whether to allow non-public IP addresses on the network
    #[serde(default)]
    pub allow_private_addresses: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mempool {
    /// time to live in the mempool before being discarded. If the value is not applied
    /// in a block within this duration it will be discarded.
    pub fragment_ttl: Duration,
    /// FragmentLog time to live, it is for information purposes, we log all the fragments
    /// logs in a cache. The log will be discarded at the end of the ttl.
    pub log_ttl: Duration,
    /// interval between 2 garbage collection check of the mempool and the log cache.
    pub garbage_collection_interval: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address(pub poldercast::Address);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Topic(pub poldercast::Topic);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterestLevel(pub poldercast::InterestLevel);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Explorer {
    pub enabled: bool,
}

impl Default for P2pConfig {
    fn default() -> Self {
        P2pConfig {
            public_address: None,
            listen_address: None,
            trusted_peers: None,
            topics_of_interest: None,
            allow_private_addresses: false,
        }
    }
}
impl Default for Mempool {
    fn default() -> Self {
        Mempool {
            fragment_ttl: Duration::new(30 * 60, 0),
            log_ttl: Duration::new(3600, 0),
            garbage_collection_interval: Duration::new(3600 / 4, 0),
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

impl Address {
    pub fn to_socketaddr(&self) -> Option<SocketAddr> {
        self.0.to_socketaddr()
    }
}

impl std::fmt::Display for Address {
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
impl Serialize for Topic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if self.0 == NEW_MESSAGES_TOPIC.into() {
            serializer.serialize_str("messages")
        } else if self.0 == NEW_BLOCKS_TOPIC.into() {
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
                    "messages" => Ok(Topic(NEW_MESSAGES_TOPIC.into())),
                    "blocks" => Ok(Topic(NEW_BLOCKS_TOPIC.into())),
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
