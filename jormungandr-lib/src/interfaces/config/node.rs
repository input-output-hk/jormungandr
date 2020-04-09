use crate::{
    interfaces::{Log, Mempool},
    time::Duration,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rest {
    pub listen: SocketAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2p {
    /// The public address to which other peers may connect to
    pub public_address: poldercast::Address,

    pub public_id: poldercast::Id,

    /// the rendezvous points for the peer to connect to in order to initiate
    /// the p2p discovery from.
    pub trusted_peers: Vec<TrustedPeer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen_address: Option<poldercast::Address>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<u32>,

    pub allow_private_addresses: bool,

    pub topics_of_interest: Option<TopicsOfInterest>,

    pub policy: Option<Policy>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub address: poldercast::Address,
    pub id: poldercast::Id,
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
