use chain_impl_mockchain::chaintypes::ConsensusVersion;
use chain_impl_mockchain::value::Value;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", remote = "ConsensusVersion")]
pub enum ConsensusVersionSerde {
    Bft,
    GenesisPraos,
}

#[derive(Deserialize)]
#[serde(remote = "Value")]
pub struct ValueSerde(pub u64);
