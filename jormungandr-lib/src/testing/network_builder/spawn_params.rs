use crate::interfaces::{
    Explorer, LayersConfig, Mempool, NodeConfig, Policy, TopicsOfInterest, TrustedPeer,
};

use super::{LeadershipMode, PersistenceMode};
use std::path::PathBuf;

#[derive(Clone)]
pub struct SpawnParams {
    pub topics_of_interest: Option<TopicsOfInterest>,
    pub explorer: Option<Explorer>,
    pub mempool: Option<Mempool>,
    pub policy: Option<Policy>,
    pub jormungandr: Option<PathBuf>,
    pub listen_address: Option<Option<poldercast::Address>>,
    pub trusted_peers: Option<Vec<TrustedPeer>>,
    pub preferred_layer: Option<LayersConfig>,
    pub leadership_mode: LeadershipMode,
    pub persistence_mode: PersistenceMode,
    pub max_connections: Option<u32>,
    pub max_inbound_connections: Option<u32>,
    pub alias: String,
    pub node_id: Option<poldercast::Address>,
}

impl SpawnParams {
    pub fn new(alias: &str) -> Self {
        Self {
            topics_of_interest: None,
            explorer: None,
            mempool: None,
            policy: None,
            jormungandr: None,
            alias: alias.to_owned(),
            leadership_mode: LeadershipMode::Leader,
            persistence_mode: PersistenceMode::Persistent,
            node_id: None,
            trusted_peers: None,
            listen_address: None,
            max_connections: None,
            max_inbound_connections: None,
            preferred_layer: None,
        }
    }

    pub fn get_alias(&self) -> String {
        self.alias.clone()
    }

    pub fn no_listen_address(&mut self) -> &mut Self {
        self.listen_address(None)
    }

    pub fn listen_address(&mut self, address: Option<poldercast::Address>) -> &mut Self {
        self.listen_address = Some(address);
        self
    }

    pub fn get_leadership_mode(&self) -> LeadershipMode {
        self.leadership_mode.clone()
    }

    pub fn get_persistence_mode(&self) -> PersistenceMode {
        self.persistence_mode.clone()
    }

    pub fn topics_of_interest(&mut self, topics_of_interest: TopicsOfInterest) -> &mut Self {
        self.topics_of_interest = Some(topics_of_interest);
        self
    }

    pub fn node_id(&mut self, node_id: poldercast::Address) -> &mut Self {
        self.node_id = Some(node_id);
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

    pub fn mempool(&mut self, mempool: Mempool) -> &mut Self {
        self.mempool = Some(mempool);
        self
    }

    pub fn trusted_peers(&mut self, trusted_peers: Vec<TrustedPeer>) -> &mut Self {
        self.trusted_peers = Some(trusted_peers);
        self
    }

    pub fn preferred_layer(&mut self, preferred_layer: LayersConfig) -> &mut Self {
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

    pub fn override_settings(&self, node_config: &mut NodeConfig) {
        if let Some(topics_of_interest) = &self.topics_of_interest {
            node_config.p2p.topics_of_interest = Some(topics_of_interest.clone());
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

        if let Some(node_id) = &self.node_id {
            node_config.p2p.public_address = node_id.clone();
        }

        if let Some(max_inbound_connections) = &self.max_inbound_connections {
            node_config.p2p.max_inbound_connections = Some(max_inbound_connections.clone());
        }

        if let Some(max_connections) = &self.max_connections {
            node_config.p2p.max_connections = Some(max_connections.clone());
        }

        if let Some(listen_address_option) = &self.listen_address {
            node_config.p2p.listen_address = listen_address_option.clone();
        }

        if let Some(trusted_peers) = &self.trusted_peers {
            node_config.p2p.trusted_peers = trusted_peers.clone();
        }

        if let Some(preferred_layer) = &self.preferred_layer {
            node_config.p2p.layers = Some(preferred_layer.clone());
        }
    }
}
