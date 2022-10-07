use crate::crypto::{hash::Hash, key::SigningKey};
use chain_crypto::{Ed25519, RistrettoGroup2HashDh, SumEd25519_12};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct NodeSecret {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bft: Option<Bft>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genesis: Option<GenesisPraos>,
}

/// hold the node's bft secret setting
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Bft {
    pub signing_key: SigningKey<Ed25519>,
}

/// the genesis praos setting
///
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenesisPraos {
    pub node_id: Hash,
    pub sig_key: SigningKey<SumEd25519_12>,
    pub vrf_key: SigningKey<RistrettoGroup2HashDh>,
}
