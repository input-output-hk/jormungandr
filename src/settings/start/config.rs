use crate::{network::p2p_topology::NodeId, settings::logging::LogFormat};
use poldercast;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use std::{collections::BTreeMap, fmt, net::SocketAddr};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub secret_file: Option<PathBuf>,
    pub genesis: Option<Genesis>,
    pub legacy_listen: Option<Vec<SocketAddr>>,
    pub grpc_listen: Option<Vec<SocketAddr>>,
    pub legacy_peers: Option<Vec<SocketAddr>>,
    pub grpc_peers: Option<Vec<SocketAddr>>,
    pub storage: Option<PathBuf>,
    pub logger: Option<ConfigLogSettings>,
    pub rest: Option<Rest>,
    pub peer_2_peer: P2pConfig,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Genesis {
    pub constant: GenesisConstants,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisConstants {
    /// stability time
    k: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigLogSettings {
    pub verbosity: Option<u8>,
    pub format: Option<LogFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rest {
    pub listen: SocketAddr,
    pub prefix: Option<String>,
    pub pkcs12: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct P2pConfig {
    /// the address to which other peers may connect to
    pub public_address: Option<Address>,
    pub public_id: Option<NodeId>,
    /// the rendezvous points for the peer to connect to in order to initiate
    /// the p2p discovery from.
    pub trusted_peers: Option<Vec<TrustedPeer>>,
    /// the topic subscriptions
    ///
    /// When connecting to different nodes we will expose these too in order to
    /// help the different modules of the P2P topology engine to determine the
    /// best possible neighborhood.
    pub topics_of_interests: Option<BTreeMap<Topic, InterestLevel>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address(pub poldercast::Address);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Topic(pub poldercast::Topic);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterestLevel(pub poldercast::InterestLevel);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub address: Address,
    pub id: NodeId,
}

impl Address {
    pub fn to_socketaddr(&self) -> Option<SocketAddr> {
        self.0.to_socketaddr()
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
        use crate::network::p2p_topology::{NEW_BLOCKS_TOPIC, NEW_MESSAGES_TOPIC};
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
                use crate::network::p2p_topology::{NEW_BLOCKS_TOPIC, NEW_MESSAGES_TOPIC};
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
