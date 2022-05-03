#![allow(dead_code)]

use jormungandr_lib::{
    interfaces::{
        Cors, JRpc, LayersConfig, Log, Mempool, NodeConfig, P2p, Policy, Rest, Tls,
        TopicsOfInterest, TrustedPeer,
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

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_RPC_THREADS_AMOUNT: usize = 1;

impl Default for NodeConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeConfigBuilder {
    pub fn new() -> NodeConfigBuilder {
        let rest_port = super::get_available_port();
        let public_address_port = super::get_available_port();
        let jrpc_port = super::get_available_port();
        let grpc_public_address: Multiaddr =
            format!("/ip4/{}/tcp/{}", DEFAULT_HOST, public_address_port)
                .parse()
                .unwrap();

        NodeConfigBuilder {
            storage: None,
            log: None,
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

    pub fn with_policy(&mut self, policy: Policy) -> &mut Self {
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
        self.p2p.listen = Some(listen_address.parse().unwrap());
        self
    }

    pub fn with_rest_tls_config(&mut self, tls: Tls) -> &mut Self {
        self.rest.tls = Some(tls);
        self
    }

    pub fn with_rest_cors_config(&mut self, cors: Cors) -> &mut Self {
        self.rest.cors = Some(cors);
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
            jrpc: self.jrpc.clone(),
            p2p: self.p2p.clone(),
            mempool: self.mempool.clone(),
            bootstrap_from_trusted_peers: Some(!self.p2p.trusted_peers.is_empty()),
            skip_bootstrap: Some(self.p2p.trusted_peers.is_empty()),
        }
    }
}
