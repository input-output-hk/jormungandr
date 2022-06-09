use assert_fs::{fixture::ChildPath, prelude::*};
use chain_core::property::FromStr;
use chain_crypto::{Ed25519, RistrettoGroup2HashDh, SumEd25519_12};
use jormungandr_lib::{
    crypto::{hash::Hash, key::SigningKey},
    interfaces::{Bft, GenesisPraos, NodeSecret},
};
use std::option::Option;

#[derive(Debug, Clone)]
pub struct SecretModelFactory {
    pub bft: Option<Bft>,
    pub genesis: Option<GenesisPraos>,
}

pub fn write_secret(node_secret: &NodeSecret, output_file: &ChildPath) {
    let content = serde_yaml::to_string(&node_secret).expect("Cannot serialize secret node model");
    output_file.write_str(&content).unwrap();
}

impl SecretModelFactory {
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
        vrf_key: SigningKey<RistrettoGroup2HashDh>,
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
