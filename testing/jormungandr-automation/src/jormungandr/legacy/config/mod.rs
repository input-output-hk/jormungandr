mod configuration_builder;
mod node;

pub use configuration_builder::{
    LegacyConfigConverter, LegacyConfigConverterError, LegacyNodeConfigConverter,
};
use jormungandr_lib::interfaces::{
    JRpc, LayersConfig, LogEntry, LogOutput, Mempool, Policy, Rest, TopicsOfInterest,
};
use multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2p {
    /// The public address to which other peers may connect to
    pub public_address: Multiaddr,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_id: Option<String>,

    /// the rendezvous points for the peer to connect to in order to initiate
    /// the p2p discovery from.
    pub trusted_peers: Vec<TrustedPeer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen: Option<SocketAddr>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_inbound_connections: Option<u32>,

    pub allow_private_addresses: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics_of_interest: Option<TopicsOfInterest>,

    pub policy: Option<Policy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<LayersConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub address: Multiaddr,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Log(pub Vec<LogEntry>);

impl From<jormungandr_lib::interfaces::Log> for Log {
    fn from(log: jormungandr_lib::interfaces::Log) -> Self {
        Self(vec![log.0])
    }
}

impl Log {
    pub fn file_path(&self) -> Option<&std::path::Path> {
        self.0.iter().find_map(|log_entry| match &log_entry.output {
            LogOutput::File(path) => Some(path.as_path()),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<PathBuf>,
    pub rest: Rest,
    pub jrpc: JRpc,
    pub p2p: P2p,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<Log>,
    #[serde(rename = "log", skip_serializing_if = "Option::is_none")]
    pub single_log: Option<jormungandr_lib::interfaces::Log>,
    pub mempool: Option<Mempool>,
    pub bootstrap_from_trusted_peers: Option<bool>,
    pub skip_bootstrap: Option<bool>,
}
