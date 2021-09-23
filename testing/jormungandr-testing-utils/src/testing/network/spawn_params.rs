use jormungandr_lib::interfaces::{
    Explorer, LayersConfig, Mempool, NodeConfig, Policy, PreferredListConfig, TopicsOfInterest,
    TrustedPeer,
};
use jormungandr_lib::time::Duration;
use log::Level;
use multiaddr::Multiaddr;
use std::net::SocketAddr;

use super::{LeadershipMode, NodeAlias, PersistenceMode};
use crate::testing::node::Version;
use serde::Deserialize;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
pub struct SpawnParams {
    alias: NodeAlias,
    bootstrap_from_peers: Option<bool>,
    explorer: Option<Explorer>,
    faketime: Option<FaketimeConfig>,
    gossip_interval: Option<Duration>,
    jormungandr: Option<PathBuf>,
    leadership_mode: LeadershipMode,
    listen_address: Option<Option<SocketAddr>>,
    log_level: Option<Level>,
    max_bootstrap_attempts: Option<usize>,
    max_connections: Option<u32>,
    max_inbound_connections: Option<u32>,
    mempool: Option<Mempool>,
    network_stuck_check: Option<Duration>,
    node_key_file: Option<PathBuf>,
    persistence_mode: PersistenceMode,
    persistent_fragment_log: Option<PathBuf>,
    policy: Option<Policy>,
    preferred_layer: Option<PreferredListConfig>,
    public_address: Option<Multiaddr>,
    skip_bootstrap: Option<bool>,
    topics_of_interest: Option<TopicsOfInterest>,
    trusted_peers: Option<Vec<TrustedPeer>>,
    version: Option<Version>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FaketimeConfig {
    /// Clock drift (1 = no drift, 2 = double speed)
    pub drift: f32,
    /// Offset from the real clock in seconds
    pub offset: i32,
}

impl SpawnParams {
    pub fn new(alias: &str) -> Self {
        Self {
            alias: alias.to_owned(),
            bootstrap_from_peers: None,
            explorer: None,
            faketime: None,
            gossip_interval: None,
            jormungandr: None,
            leadership_mode: LeadershipMode::Leader,
            listen_address: None,
            log_level: None,
            max_bootstrap_attempts: None,
            max_connections: None,
            max_inbound_connections: None,
            mempool: None,
            network_stuck_check: None,
            node_key_file: None,
            persistence_mode: PersistenceMode::Persistent,
            persistent_fragment_log: None,
            policy: None,
            preferred_layer: None,
            public_address: None,
            skip_bootstrap: None,
            topics_of_interest: None,
            trusted_peers: None,
            version: None,
        }
    }

    pub fn get_alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn no_listen_address(&mut self) -> &mut Self {
        self.listen_address(None)
    }

    pub fn listen_address(&mut self, address: Option<SocketAddr>) -> &mut Self {
        self.listen_address = Some(address);
        self
    }

    pub fn persistent_fragment_log<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.persistent_fragment_log = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn get_leadership_mode(&self) -> LeadershipMode {
        self.leadership_mode
    }

    pub fn get_persistence_mode(&self) -> PersistenceMode {
        self.persistence_mode
    }

    pub fn get_version(&self) -> &Option<Version> {
        &self.version
    }

    pub fn topics_of_interest(&mut self, topics_of_interest: TopicsOfInterest) -> &mut Self {
        self.topics_of_interest = Some(topics_of_interest);
        self
    }

    pub fn public_address(&mut self, public_address: Multiaddr) -> &mut Self {
        self.public_address = Some(public_address);
        self
    }

    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = Some(version);
        self
    }

    pub fn max_connections(&mut self, max_connections: u32) -> &mut Self {
        self.max_connections = Some(max_connections);
        self
    }

    pub fn max_inbound_connections(&mut self, max_inbound_connections: u32) -> &mut Self {
        self.max_inbound_connections = Some(max_inbound_connections);
        self
    }

    pub fn explorer(&mut self, explorer: Explorer) -> &mut Self {
        self.explorer = Some(explorer);
        self
    }

    pub fn skip_bootstrap(&mut self, skip_bootstrap: bool) -> &mut Self {
        self.skip_bootstrap = Some(skip_bootstrap);
        self
    }

    pub fn mempool(&mut self, mempool: Mempool) -> &mut Self {
        self.mempool = Some(mempool);
        self
    }

    pub fn bootstrap_from_peers(&mut self, bootstrap_from_peers: bool) -> &mut Self {
        self.bootstrap_from_peers = Some(bootstrap_from_peers);
        self
    }

    pub fn trusted_peers(&mut self, trusted_peers: Vec<TrustedPeer>) -> &mut Self {
        self.trusted_peers = Some(trusted_peers);
        self
    }

    pub fn preferred_layer(&mut self, preferred_layer: PreferredListConfig) -> &mut Self {
        self.preferred_layer = Some(preferred_layer);
        self
    }

    pub fn policy(&mut self, policy: Policy) -> &mut Self {
        self.policy = Some(policy);
        self
    }

    pub fn jormungandr(&mut self, jormungandr_app_path: PathBuf) -> &mut Self {
        self.jormungandr = Some(jormungandr_app_path);
        self
    }

    pub fn passive(&mut self) -> &mut Self {
        self.leadership_mode = LeadershipMode::Passive;
        self
    }

    pub fn leader(&mut self) -> &mut Self {
        self.leadership_mode = LeadershipMode::Leader;
        self
    }

    pub fn in_memory(&mut self) -> &mut Self {
        self.persistence_mode = PersistenceMode::InMemory;
        self
    }

    pub fn leadership_mode(&mut self, leadership_mode: LeadershipMode) -> &mut Self {
        self.leadership_mode = leadership_mode;
        self
    }

    pub fn persistence_mode(&mut self, persistence_mode: PersistenceMode) -> &mut Self {
        self.persistence_mode = persistence_mode;
        self
    }

    pub fn node_key_file(&mut self, node_key_file: PathBuf) -> &mut Self {
        self.node_key_file = Some(node_key_file);
        self
    }

    pub fn faketime(&mut self, faketime: FaketimeConfig) -> &mut Self {
        self.faketime = Some(faketime);
        self
    }

    pub fn get_faketime(&self) -> Option<&FaketimeConfig> {
        self.faketime.as_ref()
    }

    pub fn gossip_interval(&mut self, duration: Duration) -> &mut Self {
        self.gossip_interval = Some(duration);
        self
    }

    pub fn log_level(&mut self, level: Level) -> &mut Self {
        self.log_level = Some(level);
        self
    }

    pub fn get_log_level(&self) -> Option<&Level> {
        self.log_level.as_ref()
    }

    pub fn max_bootstrap_attempts(&mut self, attempts: usize) -> &mut Self {
        self.max_bootstrap_attempts = Some(attempts);
        self
    }

    pub fn network_stuck_check(&mut self, duration: Duration) -> &mut Self {
        self.network_stuck_check = Some(duration);
        self
    }

    pub fn get_jormungandr(&self) -> &Option<PathBuf> {
        &self.jormungandr
    }

    pub fn override_settings(&self, node_config: &mut NodeConfig) {
        if let Some(topics_of_interest) = &self.topics_of_interest {
            if let Some(ref mut config) = node_config.p2p.layers {
                config.topics_of_interest = Some(topics_of_interest.clone());
            } else {
                node_config.p2p.layers = Some(LayersConfig {
                    preferred_list: None,
                    topics_of_interest: Some(topics_of_interest.clone()),
                });
            }
        }

        if let Some(explorer) = &self.explorer {
            node_config.explorer = explorer.clone();
        }

        if let Some(mempool) = &self.mempool {
            node_config.mempool = Some(mempool.clone());
        }

        if let Some(policy) = &self.policy {
            node_config.p2p.policy = Some(policy.clone());
        }

        if let Some(public_address) = &self.public_address {
            node_config.p2p.public_address = public_address.clone();
        }

        if let Some(max_inbound_connections) = &self.max_inbound_connections {
            node_config.p2p.max_inbound_connections = Some(*max_inbound_connections);
        }

        if let Some(max_connections) = &self.max_connections {
            node_config.p2p.max_connections = Some(*max_connections);
        }

        if let Some(listen_address_option) = &self.listen_address {
            node_config.p2p.listen = *listen_address_option;
        }

        if let Some(trusted_peers) = &self.trusted_peers {
            node_config.p2p.trusted_peers = trusted_peers.clone();
        }

        if let Some(preferred_layer) = &self.preferred_layer {
            if let Some(ref mut config) = node_config.p2p.layers {
                config.preferred_list = Some(preferred_layer.clone());
            } else {
                node_config.p2p.layers = Some(LayersConfig {
                    preferred_list: Some(preferred_layer.clone()),
                    topics_of_interest: None,
                });
            }
        }

        if let Some(bootstrap_from_peers) = &self.bootstrap_from_peers {
            node_config.bootstrap_from_trusted_peers = Some(*bootstrap_from_peers);
        }

        if let Some(skip_bootstrap) = &self.skip_bootstrap {
            node_config.skip_bootstrap = Some(*skip_bootstrap);
        }

        if let Some(node_key_file) = &self.node_key_file {
            node_config.p2p.node_key_file = Some(node_key_file.clone());
        }

        if self.gossip_interval.is_some() {
            node_config.p2p.gossip_interval = self.gossip_interval;
        }

        if let Some(max_bootstrap_attempts) = self.max_bootstrap_attempts {
            node_config.p2p.max_bootstrap_attempts = Some(max_bootstrap_attempts);
        }

        if let Some(network_stuck_check) = self.network_stuck_check {
            node_config.p2p.network_stuck_check = Some(network_stuck_check);
        }
    }
}
