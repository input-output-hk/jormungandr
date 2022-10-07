use assert_fs::prelude::*;
use chain_core::property::FromStr;
use chain_crypto::{Ed25519, RistrettoGroup2HashDh, SumEd25519_12};
use jormungandr_lib::{
    crypto::{
        hash::Hash,
        key::{KeyPair, SigningKey},
    },
    interfaces::{Bft, GenesisPraos, NodeSecret},
};
use std::{option::Option, path::PathBuf};

#[derive(Debug, Clone, Default)]
pub struct SecretModelFactory {
    pub bft: Option<Bft>,
    pub genesis: Option<GenesisPraos>,
}

impl From<KeyPair<Ed25519>> for SecretModelFactory {
    fn from(key_pair: KeyPair<Ed25519>) -> Self {
        Self {
            bft: Some(Bft {
                signing_key: key_pair.signing_key(),
            }),
            genesis: None,
        }
    }
}

impl SecretModelFactory {
    pub fn bft(signing_key: SigningKey<Ed25519>) -> Self {
        Self {
            bft: Some(Bft { signing_key }),
            genesis: None,
        }
    }

    pub fn genesis(
        signing_key: SigningKey<SumEd25519_12>,
        vrf_key: SigningKey<RistrettoGroup2HashDh>,
        node_id: &str,
    ) -> Self {
        Self {
            genesis: Some(GenesisPraos {
                node_id: Hash::from_str(node_id).unwrap(),
                sig_key: signing_key,
                vrf_key,
            }),
            bft: None,
        }
    }

    pub fn write_to_file_if_defined(&self, temp_dir: &impl PathChild) -> Option<PathBuf> {
        let secret: NodeSecret = self.clone().into();
        secret.write_to_file_if_defined(temp_dir)
    }

    pub fn is_defined(&self) -> bool {
        self.genesis.is_some() && self.bft.is_some()
    }
}

#[allow(clippy::from_over_into)]
impl Into<NodeSecret> for SecretModelFactory {
    fn into(self) -> NodeSecret {
        NodeSecret {
            bft: self.bft,
            genesis: self.genesis,
        }
    }
}

pub trait NodeSecretExtension {
    fn write_to_file_if_defined(&self, temp_dir: &impl PathChild) -> Option<PathBuf>;
}

impl NodeSecretExtension for NodeSecret {
    fn write_to_file_if_defined(&self, temp_dir: &impl PathChild) -> Option<PathBuf> {
        if self.bft.is_some() && self.genesis.is_some() {
            panic!("both bft and genesis secrets defined!");
        }

        let output_file = temp_dir.child("node_secret.yaml");

        if self.bft.is_some() || self.genesis.is_some() {
            let content = serde_yaml::to_string(&self).expect("Cannot serialize secret node model");
            output_file.write_str(&content).unwrap();
            Some(output_file.to_path_buf())
        } else {
            None
        }
    }
}
