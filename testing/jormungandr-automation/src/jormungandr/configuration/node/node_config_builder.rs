#![allow(dead_code)]

use crate::jormungandr::get_available_port;
use jormungandr_lib::{
    interfaces::{
        Cors, JRpc, LayersConfig, Log, LogEntry, LogOutput, Mempool, NodeConfig, P2p, Policy, Rest,
        Tls, TopicsOfInterest, TrustedPeer,
    },
    time::Duration,
};
use multiaddr::Multiaddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct NodeConfigBuilder {
    pub storage: Option<PathBuf>,
    pub log: Option<Log>,
    pub rest: Rest,
    pub jrpc: JRpc,
    pub p2p: P2p,
    pub mempool: Option<Mempool>,
}

impl NodeConfigBuilder {
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log
            .as_mut()
            .expect("log not defined, so cannot set the level")
            .0
            .level = level.into();
        self
    }
}

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_RPC_THREADS_AMOUNT: usize = 1;

impl Default for NodeConfigBuilder {
    fn default() -> Self {
        let rest_port = get_available_port();
        let public_address_port = get_available_port();
        let jrpc_port = get_available_port();
        let grpc_public_address: Multiaddr =
            format!("/ip4/{}/tcp/{}", DEFAULT_HOST, public_address_port)
                .parse()
                .unwrap();

        NodeConfigBuilder {
            storage: None,
            log: Some(Log(LogEntry {
                level: "trace".to_string(),
                format: "json".to_string(),
                output: LogOutput::Stdout,
            })),
            rest: Rest {
                listen: format!("{}:{}", DEFAULT_HOST, rest_port).parse().unwrap(),
                tls: None,
                cors: None,
            },
            jrpc: JRpc {
                listen: format!("{}:{}", DEFAULT_HOST, jrpc_port).parse().unwrap(),
            },
            p2p: P2p {
                node_key_file: None,
                trusted_peers: vec![],
                public_address: grpc_public_address,
                listen: None,
                max_inbound_connections: None,
                max_connections: None,
                allow_private_addresses: true,
                policy: Some(Policy {
                    quarantine_duration: Some(Duration::new(1, 0)),
                    quarantine_whitelist: None,
                }),
                layers: Some(LayersConfig {
                    preferred_list: Default::default(),
                    topics_of_interest: Some(TopicsOfInterest {
                        messages: String::from("high"),
                        blocks: String::from("high"),
                    }),
                }),
                gossip_interval: None,
                max_bootstrap_attempts: None,
                network_stuck_check: None,
            },
            mempool: Some(Mempool::default()),
        }
    }
}

impl NodeConfigBuilder {
    pub fn with_policy(mut self, policy: Policy) -> Self {
        self.p2p.policy = Some(policy);
        self
    }

    pub fn with_log(mut self, log: Log) -> Self {
        self.log = Some(log);
        self
    }

    pub fn without_log(mut self) -> Self {
        self.log = None;
        self
    }

    pub fn with_trusted_peers(mut self, trusted_peers: Vec<TrustedPeer>) -> Self {
        self.p2p.trusted_peers = trusted_peers;
        self
    }

    pub fn with_public_address(mut self, public_address: String) -> Self {
        self.p2p.public_address = public_address.parse().unwrap();
        self
    }

    pub fn with_listen_address(mut self, listen_address: String) -> Self {
        self.p2p.listen = Some(listen_address.parse().unwrap());
        self
    }

    pub fn with_rest_tls_config(mut self, tls: Tls) -> Self {
        self.rest.tls = Some(tls);
        self
    }

    pub fn with_rest_cors_config(mut self, cors: Cors) -> Self {
        self.rest.cors = Some(cors);
        self
    }

    pub fn with_mempool(mut self, mempool: Mempool) -> Self {
        self.mempool = Some(mempool);
        self
    }

    pub fn with_storage(mut self, path: PathBuf) -> Self {
        self.storage = Some(path);
        self
    }

    pub fn build(mut self) -> NodeConfig {
        //remove id from trusted peers
        for trusted_peer in self.p2p.trusted_peers.iter_mut() {
            trusted_peer.id = None;
        }

        NodeConfig {
            bootstrap_from_trusted_peers: Some(!self.p2p.trusted_peers.is_empty()),
            skip_bootstrap: Some(self.p2p.trusted_peers.is_empty()),
            storage: self.storage,
            log: self.log,
            rest: self.rest,
            jrpc: self.jrpc,
            p2p: self.p2p,
            mempool: self.mempool,
        }
    }
}
