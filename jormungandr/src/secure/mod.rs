use chain_crypto::{Blake2b256, Curve25519_2HashDH, Ed25519, PublicKey, SumEd25519_12};
use chain_impl_mockchain::leadership::{BftLeader, GenesisLeader};
use jormungandr_lib::crypto::{
    hash::Hash,
    key::{Identifier, SigningKey},
};
use serde::Deserialize;
use std::path::Path;

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
    vrf_key: SigningKey<Curve25519_2HashDH>,
}

/// the genesis praos setting
///
#[derive(Clone, Deserialize)]
pub struct GenesisPraosPublic {
    sig_key: Identifier<SumEd25519_12>,
    vrf_key: Identifier<Curve25519_2HashDH>,
}

#[derive(Clone, Deserialize)]
pub struct OwnerKey(Identifier<Ed25519>);

#[derive(Clone, Deserialize)]
pub struct StakePoolInfo {
    serial: u128,
    owners: Vec<OwnerKey>,
    initial_key: GenesisPraosPublic,
}

/// Node Secret(s)
#[derive(Clone, Deserialize)]
pub struct NodeSecret {
    pub bft: Option<Bft>,
    pub genesis: Option<GenesisPraos>,
}

/// Node Secret's Public parts
#[derive(Clone)]
pub struct NodePublic {
    pub block_publickey: PublicKey<Ed25519>,
}

custom_error! {pub NodeSecretFromFileError
    Io { source: std::io::Error } = "Cannot read node's secrets: {source}",
    Format { source: serde_yaml::Error } = "Invalid Node secret file: {source}",
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
}
