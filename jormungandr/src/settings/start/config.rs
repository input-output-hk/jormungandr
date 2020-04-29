use crate::{
    network::p2p::{layers::LayersConfig, topic, Address, PolicyConfig},
    settings::logging::{LogFormat, LogOutput},
    settings::LOG_FILTER_LEVEL_POSSIBLE_VALUES,
};
use jormungandr_lib::{interfaces::Mempool, time::Duration};
use poldercast;
use serde::{de::Error as _, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use slog::FilterLevel;

use std::{collections::BTreeMap, fmt, net::SocketAddr, path::PathBuf, str::FromStr};

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

    #[serde(default)]
    pub http_fetch_block0_service: Vec<String>,

    pub explorer: Option<Explorer>,

    /// the time interval with no blockchain updates after which alerts are thrown
    #[serde(default)]
    pub no_blockchain_updates_warning_interval: Option<Duration>,

    pub bootstrap_from_trusted_peers: Option<bool>,
    pub skip_bootstrap: Option<bool>,
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
    /// Enables TLS and disables plain HTTP if provided
    pub tls: Option<Tls>,
    /// Enables CORS if provided
    pub cors: Option<Cors>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Tls {
    /// Path to server X.509 certificate chain file, must be PEM-encoded and contain at least 1 item
    pub cert_file: String,
    /// Path to server private key file, must be PKCS8 with single PEM-encoded, unencrypted key
    pub priv_key_file: String,
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

    /// keep the public id there and present, but yet make it optional as it is
    /// no longer needed.
    ///
    /// TODO: To remove once we can afford a breaking change in the config
    #[serde(default, skip)]
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

    /// If this value is set, it will trigger a force reset of the topology
    /// layers. The default is to not do force the reset. It is recommended
    /// to let the protocol handle it.
    ///
    #[serde(default)]
    pub topology_force_reset_interval: Option<Duration>,

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedPeer {
    pub address: Address,

    // KEEP the ID optional, this is no longer needed but removing this will
    // allow to keep some back compatibility.
    //
    // TODO: to remove once we can afford having a config breaking change
    #[serde(skip, default)]
    pub id: Option<Id>,
}

// Lifted from poldercast 0.11 for backward compatibility
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id([u8; ID_LEN]);

const ID_LEN: usize = 24;

impl Id {
    fn zero() -> Self {
        Id([0; ID_LEN])
    }
}

impl FromStr for Id {
    type Err = hex::FromHexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut v = Self::zero();
        hex::decode_to_slice(s, &mut v.0)?;
        Ok(v)
    }
}

impl AsRef<[u8]> for Id {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Id").field(&hex::encode(self)).finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Leadership {
    /// the number of entries allowed in the leadership logs, beyond this point
    /// the least recently used log will be erased from the logs for a new one
    /// to be inserted.
    pub logs_capacity: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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
            max_inbound_connections: None,
            max_connections_threshold: None,
            allow_private_addresses: false,
            policy: PolicyConfig::default(),
            layers: LayersConfig::default(),
            max_unreachable_nodes_to_connect_per_event: None,
            gossip_interval: None,
            topology_force_reset_interval: None,
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

impl std::str::FromStr for TrustedPeer {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('@');

        let address = if let Some(address) = split.next() {
            address
                .parse::<poldercast::Address>()
                .map_err(|e| e.to_string())?
        } else {
            return Err("Missing address component".to_owned());
        };

        let optional_id = if let Some(id) = split.next() {
            let id = id.parse::<Id>().map_err(|e| e.to_string())?;
            Some(id)
        } else {
            None
        };

        Ok(TrustedPeer {
            address,
            id: optional_id,
        })
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
