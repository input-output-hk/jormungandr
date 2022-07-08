use chain_crypto::{Blake2b256, Ed25519, PublicKey, RistrettoGroup2HashDh, SumEd25519_12};
use chain_impl_mockchain::leadership::{BftLeader, GenesisLeader};
use jormungandr_lib::crypto::{
    hash::Hash,
    key::{Identifier, SigningKey},
};
use serde::Deserialize;
use std::path::Path;
use thiserror::Error;

pub mod enclave;

/// hold the node's bft secret setting
#[derive(Clone, Deserialize)]
pub struct Bft {
    signing_key: SigningKey<Ed25519>,
}

/// the genesis praos setting
///
#[derive(Clone, Deserialize)]
pub struct GenesisPraos {
    node_id: Hash,
    sig_key: SigningKey<SumEd25519_12>,
    vrf_key: SigningKey<RistrettoGroup2HashDh>,
}

#[derive(Clone, Deserialize)]
pub struct OwnerKey(Identifier<Ed25519>);

/// Node Secret(s)
#[derive(Clone, Deserialize)]
pub struct NodeSecret {
    bft: Option<Bft>,
    genesis: Option<GenesisPraos>,
    #[cfg(feature = "evm")]
    evm_keys: Option<Vec<chain_evm::ethereum_types::H256>>,
}

/// Node Secret's Public parts
#[derive(Clone)]
pub struct NodePublic {
    pub block_publickey: PublicKey<Ed25519>,
}

#[derive(Debug, Error)]
pub enum NodeSecretFromFileError {
    #[error("Cannot read node's secrets: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid Node secret file: {0}")]
    Format(#[from] serde_yaml::Error),
}

impl NodeSecret {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<NodeSecret, NodeSecretFromFileError> {
        let file = std::fs::File::open(path)?;
        Ok(serde_yaml::from_reader(file)?)
    }

    pub fn bft(&self) -> Option<BftLeader> {
        self.bft.clone().map(|bft| BftLeader {
            sig_key: bft.signing_key.into_secret_key(),
        })
    }

    pub fn genesis(&self) -> Option<GenesisLeader> {
        self.genesis.clone().map(|genesis| GenesisLeader {
            node_id: Blake2b256::from(genesis.node_id).into(),
            sig_key: genesis.sig_key.into_secret_key(),
            vrf_key: genesis.vrf_key.into_secret_key(),
        })
    }

    #[cfg(feature = "evm")]
    pub fn evm_keys(&self) -> Vec<chain_evm::util::Secret> {
        self.evm_keys
            .as_ref()
            .map(|keys| {
                keys.iter()
                    .map(chain_evm::util::Secret::from_hash)
                    .collect()
            })
            .unwrap_or_default()
    }
}
