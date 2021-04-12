use crate::{
    network::p2p::{layers::LayersConfig, topic, Address, PolicyConfig},
    settings::logging::{LogFormat, LogOutput},
    settings::LOG_FILTER_LEVEL_POSSIBLE_VALUES,
};
pub use jormungandr_lib::interfaces::{Cors, Rest, Tls, TrustedPeer};
use jormungandr_lib::{interfaces::Mempool, time::Duration};

use multiaddr::Multiaddr;
use serde::{de::Error as _, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use tracing::level_filters::LevelFilter;

use std::{collections::BTreeMap, fmt, path::PathBuf};

#[derive(Debug, Deserialize)]
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

    #[serde(default)]
    pub http_fetch_block0_service: Vec<String>,

    pub explorer: Option<Explorer>,

    /// the time interval with no blockchain updates after which alerts are thrown
    #[serde(default)]
    pub no_blockchain_updates_warning_interval: Option<Duration>,

    #[serde(default)]
    pub bootstrap_from_trusted_peers: bool,

    #[serde(default)]
    pub skip_bootstrap: bool,

    pub block_hard_deadline: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigLogSettings {
    #[serde(with = "filter_level_opt_serde")]
    pub level: Option<LevelFilter>,
    pub format: Option<LogFormat>,
    pub output: Option<LogOutput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct P2pConfig {
    /// The public address to which other peers may connect to
    pub public_address: Option<Multiaddr>,

    /// The socket address to listen on, if different from the public address.
    /// The format is "{ip_address}:{port}".
    /// The IP address can be specified as 0.0.0.0 or :: to listen on
    /// all network interfaces.
    pub listen: Option<Address>,

    /// File with the secret key used to advertise and authenticate the node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_key_file: Option<PathBuf>,

    /// the rendezvous points for the peer to connect to in order to initiate
    /// the p2p discovery from.
    pub trusted_peers: Option<Vec<TrustedPeer>>,

    /// the topic subscriptions
    ///
    /// When connecting to different nodes we will expose these too in order to
    /// help the different modules of the P2P topology engine to determine the
    /// best possible neighborhood.
    // FIXME: Until we add a custom ring layer to poldercast this is rather useless
    // keep this around for future decisions and compatibility
    pub topics_of_interest: Option<BTreeMap<Topic, InterestLevel>>,

    /// Limit on the number of simultaneous connections.
    /// If not specified, an internal default limit is used.
    pub max_connections: Option<usize>,

    /// Limit on the number of simultaneous client connections.
    /// If not specified, an internal default limit is used.
    #[serde(alias = "max_client_connections")]
    pub max_inbound_connections: Option<usize>,

    /// This setting is not used and is left for backward compatibility.
    pub max_connections_threshold: Option<usize>,

    /// Whether to allow non-public IP addresses on the network.
    /// The default is to not allow advertising non-public IP addresses.
    #[serde(default)]
    pub allow_private_addresses: bool,

    /// setting for the policy
    #[serde(default)]
    pub policy: PolicyConfig,

    /// settings for the different custom layers
    // TODO: actually implement those custom layers
    #[serde(default)]
    pub layers: LayersConfig,

    /// set the maximum number of unreachable nodes to contact at a time for every
    /// new notification. The default value is 20.
    ///
    /// Every time a new propagation event is triggered, the node will select
    /// randomly a certain amount of unreachable nodes to connect to in addition
    /// to the one selected by other p2p topology layer.
    #[serde(default)]
    pub max_unreachable_nodes_to_connect_per_event: Option<usize>,

    /// interval to start gossiping with new nodes, changing the value will
    /// affect the bandwidth. The more often the node will gossip the more
    /// bandwidth the node will need. The less often the node gossips the less
    /// good the resilience to node churn.
    ///
    /// The default value is 10seconds.
    #[serde(default)]
    pub gossip_interval: Option<Duration>,

    /// The number of times to retry bootstrapping from trusted peers. The default
    /// value of None will result in the bootstrap process retrying indefinitely. A
    /// value of zero will skip bootstrap all together -- even if trusted peers are
    /// defined. If the node fails to bootstrap from any of the trusted peers and the
    /// number of bootstrap retry attempts is exceeded, then the node will continue to
    /// run without completing the bootstrap process. This will allow the node to act
    /// as the first node in the p2p network (i.e. genesis node), or immediately begin
    /// gossip with the trusted peers if any are defined.
    #[serde(default)]
    pub max_bootstrap_attempts: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Leadership {
    /// the number of entries allowed in the leadership logs, beyond this point
    /// the least recently used log will be erased from the logs for a new one
    /// to be inserted.
    pub logs_capacity: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Topic(pub poldercast::Topic);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterestLevel(pub poldercast::InterestLevel);

impl InterestLevel {
    pub const LOW: InterestLevel = InterestLevel(poldercast::InterestLevel::new(1));
    pub const NORMAL: InterestLevel = InterestLevel(poldercast::InterestLevel::new(3));
    pub const HIGH: InterestLevel = InterestLevel(poldercast::InterestLevel::new(5));
}

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
            InterestLevel(poldercast::InterestLevel::new(1)),
        ),
        (
            Topic(topic::BLOCKS),
            InterestLevel(poldercast::InterestLevel::new(3)),
        ),
    ])
}

impl Default for P2pConfig {
    fn default() -> Self {
        P2pConfig {
            public_address: None,
            listen: None,
            node_key_file: None,
            trusted_peers: None,
            topics_of_interest: None,
            max_connections: None,
            max_inbound_connections: None,
            max_connections_threshold: None,
            allow_private_addresses: false,
            policy: PolicyConfig::default(),
            layers: LayersConfig::default(),
            max_unreachable_nodes_to_connect_per_event: None,
            gossip_interval: None,
            max_bootstrap_attempts: None,
        }
    }
}

impl Default for Leadership {
    fn default() -> Self {
        Leadership {
            logs_capacity: 1_024,
        }
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

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use serde::de::Unexpected;

                match v {
                    "messages" => Ok(Topic(topic::MESSAGES)),
                    "blocks" => Ok(Topic(topic::BLOCKS)),
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

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use serde::de::Unexpected;

                match v {
                    "low" => Ok(InterestLevel::LOW),
                    "normal" => Ok(InterestLevel::NORMAL),
                    "high" => Ok(InterestLevel::HIGH),
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
    ) -> Result<Option<LevelFilter>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(|variant| {
                variant.parse().map_err(|_| {
                    D::Error::unknown_variant(&variant, &**LOG_FILTER_LEVEL_POSSIBLE_VALUES)
                })
            })
            .transpose()
    }

    pub fn serialize<S: Serializer>(
        data: &Option<LevelFilter>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        data.map(|level| level.to_string()).serialize(serializer)
    }
}
