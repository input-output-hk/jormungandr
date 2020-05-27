#![allow(dead_code)]

use crate::common::file_utils;
use std::option::Option;
use std::path::PathBuf;

use chain_core::property::FromStr;
use chain_crypto::{Curve25519_2HashDH, Ed25519, SumEd25519_12};
use jormungandr_lib::{
    crypto::{hash::Hash, key::SigningKey},
    interfaces::{Bft, GenesisPraos, NodeSecret},
};

#[derive(Debug, Clone)]
pub struct SecretModelFactory {
    pub bft: Option<Bft>,
    pub genesis: Option<GenesisPraos>,
}

impl SecretModelFactory {
    pub fn serialize(node_secret: &NodeSecret) -> PathBuf {
        let content =
            serde_yaml::to_string(&node_secret).expect("Cannot serialize secret node model");
        file_utils::create_file_in_temp("node.secret", &content)
    }

    pub fn empty() -> NodeSecret {
        NodeSecret {
            bft: None,
            genesis: None,
        }
    }

    pub fn bft(signing_key: SigningKey<Ed25519>) -> NodeSecret {
        NodeSecret {
            bft: Some(Bft { signing_key }),
            genesis: None,
        }
    }

    pub fn genesis(
        signing_key: SigningKey<SumEd25519_12>,
        vrf_key: SigningKey<Curve25519_2HashDH>,
        node_id: &str,
    ) -> NodeSecret {
        NodeSecret {
            genesis: Some(GenesisPraos {
                node_id: Hash::from_str(node_id).unwrap(),
                sig_key: signing_key,
                vrf_key,
            }),
            bft: None,
        }
    }
}
