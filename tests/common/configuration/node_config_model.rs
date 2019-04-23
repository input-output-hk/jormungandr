extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

use super::file_utils;

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
    pub public_access: String,
    pub topics_of_interests: TopicsOfInterests,
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

impl NodeConfig {
    pub fn serialize(node_config: NodeConfig) -> PathBuf {
        let content = serde_yaml::to_string(&node_config).unwrap();
        let node_config_file_path = file_utils::create_file_in_temp("node.config", &content);
        node_config_file_path
    }

    pub fn new() -> NodeConfig {
        NodeConfig {
            storage: String::from("/tmp/storage"),
            logger: Logger {
                verbosity: 1,
                format: String::from("json"),
            },
            rest: Rest {
                listen: String::from("127.0.0.1:8443"),
                prefix: String::from("api"),
            },
            peer_2_peer: Peer2Peer {
                public_access: String::from("/ip4/127.0.0.1/tcp/8080"),
                topics_of_interests: TopicsOfInterests {
                    messages: String::from("low"),
                    blocks: String::from("normal"),
                },
            },
        }
    }
}
