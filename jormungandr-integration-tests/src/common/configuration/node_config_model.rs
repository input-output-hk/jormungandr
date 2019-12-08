#![allow(dead_code)]

extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};
use super::file_utils;
use std::path::PathBuf;

use jormungandr_lib::interfaces::Mempool;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Log(pub Vec<LogEntry>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub level: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rest {
    pub listen: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Peer2Peer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trusted_peers: Option<Vec<TrustedPeer>>,
    pub public_address: String,
    pub public_id: String,
    pub listen_address: String,
    pub topics_of_interest: TopicsOfInterest,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrustedPeer {
    pub address: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopicsOfInterest {
    pub messages: String,
    pub blocks: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<Log>,
    pub rest: Option<Rest>,
    pub p2p: Peer2Peer,
    pub mempool: Mempool,
}

const DEFAULT_HOST: &str = "127.0.0.1";

impl NodeConfig {
    pub fn serialize(node_config: &NodeConfig) -> PathBuf {
        let content = serde_yaml::to_string(&node_config).expect("Canot serialize node config");
        let node_config_file_path = file_utils::create_file_in_temp("node.config", &content);
        node_config_file_path
    }

    pub fn new() -> NodeConfig {
        let rest_port = super::get_available_port();
        let public_address_port = super::get_available_port();
        let storage_file = file_utils::get_path_in_temp("storage");
        let public_id = poldercast::Id::generate(&mut rand::rngs::OsRng::new().unwrap());
        let log = Log(vec![LogEntry {
            level: Some("info".to_string()),
            format: Some("json".to_string()),
        }]);
        let grpc_address = format!(
            "/ip4/{}/tcp/{}",
            DEFAULT_HOST,
            public_address_port.to_string()
        );

        NodeConfig {
            storage: Some(String::from(storage_file.as_os_str().to_str().unwrap())),
            log: Some(log),
            rest: Some(Rest {
                listen: format!("{}:{}", DEFAULT_HOST, rest_port.to_string()),
            }),
            p2p: Peer2Peer {
                trusted_peers: None,
                public_address: grpc_address.clone(),
                public_id: public_id.to_string(),
                listen_address: grpc_address.clone(),
                topics_of_interest: TopicsOfInterest {
                    messages: String::from("high"),
                    blocks: String::from("high"),
                },
            },
            mempool: Mempool::default(),
        }
    }

    pub fn get_p2p_port(&self) -> u16 {
        let tokens: Vec<&str> = self.p2p.public_address.as_str().split("/").collect();
        let port_str = tokens
            .get(4)
            .expect("cannot extract port from p2p.public_address");
        port_str.parse().unwrap()
    }

    pub fn regenerate_ports(&mut self) {
        self.rest.as_mut().unwrap().listen =
            format!("127.0.0.1:{}", super::get_available_port().to_string()).to_string();
        self.p2p.public_address = format!(
            "/ip4/127.0.0.1/tcp/{}",
            super::get_available_port().to_string()
        );
        self.p2p.listen_address = self.p2p.public_address.clone();
    }

    pub fn get_node_address(&self) -> String {
        let rest = self.rest.as_ref();
        let output = format!("http://{}/api", rest.unwrap().listen);
        output
    }
}
