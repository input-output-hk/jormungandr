#![allow(dead_code)]

use super::file_utils;
use std::path::PathBuf;

use jormungandr_lib::{
    interfaces::{
        Explorer, Log, LogEntry, LogOutput, Mempool, NodeConfig, P2p, Policy, Rest,
        TopicsOfInterest, TrustedPeer,
    },
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct NodeConfigBuilder {
    pub storage: Option<PathBuf>,
    pub log: Option<Log>,
    pub rest: Rest,
    pub p2p: P2p,
    pub mempool: Option<Mempool>,
    pub explorer: Explorer,
}

const DEFAULT_HOST: &str = "127.0.0.1";

impl NodeConfigBuilder {
    pub fn new() -> NodeConfigBuilder {
        let rest_port = super::get_available_port();
        let public_address_port = super::get_available_port();
        let storage_file = file_utils::get_path_in_temp("storage");
        let public_id = poldercast::Id::generate(rand::rngs::OsRng);
        let log = Some(Log(vec![LogEntry {
            level: "info".to_string(),
            format: "json".to_string(),
            output: LogOutput::Stderr,
        }]));
        let grpc_public_address: poldercast::Address = format!(
            "/ip4/{}/tcp/{}",
            DEFAULT_HOST,
            public_address_port.to_string()
        )
        .parse()
        .unwrap();

        let grpc_listen_address = grpc_public_address.clone();

        NodeConfigBuilder {
            storage: Some(storage_file),
            log: log,
            rest: Rest {
                listen: format!("{}:{}", DEFAULT_HOST, rest_port.to_string())
                    .parse()
                    .unwrap(),
            },
            p2p: P2p {
                trusted_peers: vec![],
                public_address: grpc_public_address,
                public_id: public_id.clone(),
                listen_address: grpc_listen_address,
                topics_of_interest: Some(TopicsOfInterest {
                    messages: String::from("high"),
                    blocks: String::from("high"),
                }),
                allow_private_addresses: false,
                policy: Some(Policy {
                    quarantine_duration: Duration::new(1, 0),
                }),
            },
            mempool: Some(Mempool::default()),
            explorer: Explorer { enabled: false },
        }
    }

    pub fn serialize(node_config: &NodeConfig) -> PathBuf {
        let content = serde_yaml::to_string(&node_config).expect("Canot serialize node config");
        let node_config_file_path = file_utils::create_file_in_temp("node.config", &content);
        node_config_file_path
    }

    pub fn with_explorer(&mut self) -> &mut Self {
        self.explorer.enabled = true;
        self
    }

    pub fn with_quarantine_policy(&mut self, policy: Policy) -> &mut Self {
        self.p2p.policy = Some(policy);
        self
    }

    pub fn with_log(&mut self, log: Log) -> &mut Self {
        self.log = Some(log);
        self
    }

    pub fn with_trusted_peers(&mut self, trusted_peers: Vec<TrustedPeer>) -> &mut Self {
        self.p2p.trusted_peers = trusted_peers;
        self
    }

    pub fn with_public_address(&mut self, public_address: String) -> &mut Self {
        self.p2p.public_address = public_address.parse().unwrap();
        self
    }

    pub fn with_listen_address(&mut self, listen_address: String) -> &mut Self {
        self.p2p.listen_address = listen_address.parse().unwrap();
        self
    }

    pub fn with_mempool(&mut self, mempool: Mempool) -> &mut Self {
        self.mempool = Some(mempool);
        self
    }

    pub fn with_storage(&mut self, path: PathBuf) -> &mut Self {
        self.storage = Some(path);
        self
    }

    pub fn build(&self) -> NodeConfig {
        NodeConfig {
            storage: self.storage.clone(),
            log: self.log.clone(),
            rest: self.rest.clone(),
            p2p: self.p2p.clone(),
            mempool: self.mempool.clone(),
            explorer: self.explorer.clone(),
            bootstrap_from_trusted_peers: Some(!self.p2p.trusted_peers.is_empty()),
            skip_bootstrap: Some(self.p2p.trusted_peers.is_empty()),
        }
    }
}
