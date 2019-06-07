#![allow(dead_code)]

extern crate rand;
extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};
use crate::common::file_utils;
use std::option::Option;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct SecretModel {
    pub bft: Option<BFT>,
    pub genesis: Option<Genesis>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BFT {
    pub signing_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Genesis {
    pub sig_key: String,
    pub vrf_key: String,
    pub node_id: String,
}

impl SecretModel {
    pub fn serialize(secret_model: &SecretModel) -> PathBuf {
        let content = serde_yaml::to_string(&secret_model).expect("Canot serialize secret model");
        let node_config_file_path = file_utils::create_file_in_temp("node.secret", &content);
        node_config_file_path
    }

    pub fn empty() -> Self {
        SecretModel {
            bft: None,
            genesis: None,
        }
    }

    pub fn new_bft(signing_key: &str) -> Self {
        SecretModel {
            bft: Some(BFT {
                signing_key: signing_key.to_string(),
            }),
            genesis: None,
        }
    }

    pub fn new_genesis(signing_key: &str, vrf_key: &str, node_id: &str) -> Self {
        SecretModel {
            genesis: Some(Genesis {
                sig_key: signing_key.to_string(),
                vrf_key: vrf_key.to_string(),
                node_id: node_id.to_string(),
            }),
            bft: None,
        }
    }
}
