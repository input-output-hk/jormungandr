#![allow(dead_code)]

extern crate lazy_static;
extern crate rand;
extern crate serde_derive;
use self::lazy_static::lazy_static;
use self::serde_derive::{Deserialize, Serialize};
use super::file_utils;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};

#[derive(Debug, Serialize, Deserialize)]
pub struct Logger {
    pub verbosity: i32,
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rest {
    pub listen: String,
    pub prefix: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Peer2Peer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trusted_peers: Option<Vec<Peer>>,
    pub public_address: String,
    pub topics_of_interests: TopicsOfInterests,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Peer {
    pub id: i32,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopicsOfInterests {
    pub messages: String,
    pub blocks: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    pub storage: String,
    pub logger: Logger,
    pub rest: Rest,
    pub peer_2_peer: Peer2Peer,
}

lazy_static! {
    static ref NEXT_AVAILABLE_PORT_NUMBER: AtomicU16 = AtomicU16::new(8000);
}

impl NodeConfig {
    pub fn serialize(node_config: &NodeConfig) -> PathBuf {
        let content = serde_yaml::to_string(&node_config).expect("Canot serialize node config");
        let node_config_file_path = file_utils::create_file_in_temp("node.config", &content);
        node_config_file_path
    }

    pub fn new() -> NodeConfig {
        let rest_port = NodeConfig::get_available_port();
        let public_address_port = NodeConfig::get_available_port();
        let storage_file = file_utils::get_path_in_temp("storage");

        NodeConfig {
            storage: String::from(storage_file.as_os_str().to_str().unwrap()),
            logger: Logger {
                verbosity: 1,
                format: String::from("json"),
            },
            rest: Rest {
                listen: format!("127.0.0.1:{}", rest_port.to_string()),
                prefix: String::from("api"),
            },
            peer_2_peer: Peer2Peer {
                trusted_peers: None,
                public_address: format!("/ip4/127.0.0.1/tcp/{}", public_address_port.to_string()),
                topics_of_interests: TopicsOfInterests {
                    messages: String::from("high"),
                    blocks: String::from("high"),
                },
            },
        }
    }

    pub fn regenerate_ports(&mut self) {
        self.rest.listen = format!("127.0.0.1:{}", NodeConfig::get_available_port().to_string());
        self.peer_2_peer.public_address = format!(
            "/ip4/127.0.0.1/tcp/{}",
            NodeConfig::get_available_port().to_string()
        );
    }

    pub fn get_node_address(&self) -> String {
        let output = format!("http://{}/{}", self.rest.listen, self.rest.prefix);
        output
    }

    fn get_available_port() -> u16 {
        NEXT_AVAILABLE_PORT_NUMBER.fetch_add(1, Ordering::SeqCst)
    }
}
