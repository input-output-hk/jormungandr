#![allow(dead_code)]

extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};
use super::file_utils;
use std::path::PathBuf;
#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<Logger>,
    pub rest: Option<Rest>,
    pub peer_2_peer: Peer2Peer,
}

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

        NodeConfig {
            storage: Some(String::from(storage_file.as_os_str().to_str().unwrap())),
            logger: Some(Logger {
                verbosity: 1,
                format: String::from("json"),
            }),
            rest: Some(Rest {
                listen: format!("127.0.0.1:{}", rest_port.to_string()),
                prefix: String::from("api"),
            }),
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
        self.rest.as_mut().unwrap().listen =
            format!("127.0.0.1:{}", super::get_available_port().to_string()).to_string();
        self.peer_2_peer.public_address = format!(
            "/ip4/127.0.0.1/tcp/{}",
            super::get_available_port().to_string()
        );
    }

    pub fn get_node_address(&self) -> String {
        let rest = self.rest.as_ref();
        let output = format!("http://{}/{}", rest.unwrap().listen, rest.unwrap().prefix);
        output
    }
}
