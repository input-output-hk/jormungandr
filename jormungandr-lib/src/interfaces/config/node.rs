use crate::{
    interfaces::{Log, Mempool},
    time::Duration,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
const DEFAULT_PREFERRED_VIEW_MAX: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rest {
    pub listen: SocketAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2p {
    /// The public address to which other peers may connect to
    pub public_address: poldercast::Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_id: Option<poldercast::Id>,
    /// the rendezvous points for the peer to connect to in order to initiate
    /// the p2p discovery from.
    pub trusted_peers: Vec<TrustedPeer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen_address: Option<poldercast::Address>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_inbound_connections: Option<u32>,

    pub allow_private_addresses: bool,

    pub topics_of_interest: Option<TopicsOfInterest>,

    pub policy: Option<Policy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<LayersConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopicsOfInterest {
    pub messages: String,
    pub blocks: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Policy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarantine_duration: Option<Duration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarantine_whitelist: Option<Vec<poldercast::Address>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Explorer {
    pub enabled: bool,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayersConfig {
    #[serde(default)]
    pub preferred_list: PreferredListConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PreferredViewMax(usize);

impl Default for PreferredViewMax {
    fn default() -> Self {
        Self(DEFAULT_PREFERRED_VIEW_MAX)
    }
}

impl From<PreferredViewMax> for usize {
    fn from(pvm: PreferredViewMax) -> Self {
        pvm.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PreferredListConfig {
    #[serde(default)]
    pub view_max: PreferredViewMax,

    #[serde(default)]
    // peers: HashSet<Address>,
    pub peers: Vec<TrustedPeer>,
}

/// TODO: this structure is needed only temporarily, once we have
///       have poldercast `0.13.x` we only need the address
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedPeer {
    pub address: poldercast::Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<poldercast::Id>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<PathBuf>,
    pub rest: Rest,
    pub p2p: P2p,
    pub log: Option<Log>,
    pub explorer: Explorer,
    pub mempool: Option<Mempool>,
    pub bootstrap_from_trusted_peers: Option<bool>,
    pub skip_bootstrap: Option<bool>,
}

impl P2p {
    pub fn make_trusted_peer_setting(&self) -> TrustedPeer {
        TrustedPeer {
            address: self.get_listen_address(),
            id: self.public_id.clone(),
        }
    }

    pub fn get_listen_address(&self) -> poldercast::Address {
        if let Some(listen_address) = self.listen_address.clone() {
            return listen_address;
        }
        self.public_address.clone()
    }
}
