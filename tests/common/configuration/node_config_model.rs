#![allow(dead_code)]

extern crate rand;
extern crate serde_derive;
use self::rand::Rng;
use self::serde_derive::{Deserialize, Serialize};
use super::file_utils;
use std::net::TcpListener;
use std::path::PathBuf;

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
    pub public_address: String,
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
    pub fn serialize(node_config: &NodeConfig) -> PathBuf {
        let content = serde_yaml::to_string(&node_config).expect("Canot serialize node config");
        let node_config_file_path = file_utils::create_file_in_temp("node.config", &content);
        node_config_file_path
    }

    pub fn new() -> NodeConfig {
        let rest_port = get_available_port();
        let public_address_port = get_available_port();
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
                public_address: format!("/ip4/127.0.0.1/tcp/{}", public_address_port.to_string()),
                topics_of_interests: TopicsOfInterests {
                    messages: String::from("low"),
                    blocks: String::from("normal"),
                },
            },
        }
    }

    pub fn get_node_address(&self) -> String {
        let output = format!("http://{}/{}", self.rest.listen, self.rest.prefix);
        output
    }
}

fn get_available_port() -> u16 {
    let available_port = loop {
        let port = rand::thread_rng().gen_range(8000, 9999);
        if port_is_available(port) {
            break port;
        }
    };
    available_port
}

fn port_is_available(port: u16) -> bool {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}
