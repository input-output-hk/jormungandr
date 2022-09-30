use crate::{
    network::p2p::Address,
    settings::{
        logging::{LogFormat, LogOutput},
        LOG_FILTER_LEVEL_POSSIBLE_VALUES,
    },
    topology::QuarantineConfig,
};
pub use jormungandr_lib::interfaces::{Cors, JRpc, LayersConfig, Rest, Tls, TrustedPeer};
use jormungandr_lib::{interfaces::Mempool, time::Duration};
use multiaddr::Multiaddr;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub secret_file: Option<PathBuf>,
    pub storage: Option<PathBuf>,
    pub log: Option<ConfigLogSettings>,

    /// setting of the mempool, fragment logs and related data
    #[serde(default)]
    pub mempool: Mempool,

    #[serde(default)]
    pub leadership: Leadership,

    pub rest: Option<Rest>,

    pub jrpc: Option<JRpc>,

    #[serde(default)]
    pub p2p: P2pConfig,

    #[serde(default)]
    pub http_fetch_block0_service: Vec<String>,

    #[cfg(feature = "prometheus-metrics")]
    pub prometheus: Option<Prometheus>,

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

#[derive(Debug, Clone, Deserialize, Default)]
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

    /// Limit on the number of simultaneous connections.
    /// If not specified, an internal default limit is used.
    pub max_connections: Option<usize>,

    /// Limit on the number of simultaneous client connections.
    /// If not specified, an internal default limit is used.
    pub max_client_connections: Option<usize>,

    /// This setting is not used and is left for backward compatibility.
    pub max_connections_threshold: Option<usize>,

    /// Whether to allow non-public IP addresses on the network.
    /// The default is to not allow advertising non-public IP addresses.
    #[serde(default)]
    pub allow_private_addresses: bool,

    /// setting for the policy
    #[serde(default)]
    pub policy: QuarantineConfig,

    /// settings for the different custom layers
    #[serde(default)]
    pub layers: LayersConfig,

    /// interval to start gossiping with new nodes, changing the value will
    /// affect the bandwidth. The more often the node will gossip the more
    /// bandwidth the node will need. The less often the node gossips the less
    /// good the resilience to node churn.
    ///
    /// The default value is 10seconds.
    #[serde(default)]
    pub gossip_interval: Option<Duration>,

    /// if no gossip has been received in the last interval, try to connect
    /// to nodes that were previously known to this node.
    ///
    /// The default value is 5 min.
    #[serde(default)]
    pub network_stuck_check: Option<Duration>,

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Prometheus {
    pub enabled: bool,
}

impl Default for Leadership {
    fn default() -> Self {
        Leadership {
            logs_capacity: 1_024,
        }
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
                    D::Error::unknown_variant(&variant, &LOG_FILTER_LEVEL_POSSIBLE_VALUES)
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
