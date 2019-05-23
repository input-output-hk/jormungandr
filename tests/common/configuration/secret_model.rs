#![allow(dead_code)]

extern crate rand;
extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};
use common::file_utils;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct SecretModel {
    pub bft: BFT,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BFT {
    pub signing_key: String,
}

impl SecretModel {
    pub fn serialize(secret_model: &SecretModel) -> PathBuf {
        let content = serde_yaml::to_string(&secret_model).expect("Canot serialize secret model");
        let node_config_file_path = file_utils::create_file_in_temp("node.secret", &content);
        node_config_file_path
    }

    pub fn empty() -> Self {
        SecretModel::new("")
    }

    pub fn new(signing_key: &str) -> Self {
        SecretModel {
            bft: BFT {
                signing_key: signing_key.to_string(),
            },
        }
    }
}
