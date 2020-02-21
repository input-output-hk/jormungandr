use crate::interfaces::{TaxTypeSerde, ValueDef};
use chain_impl_mockchain::value::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StakePoolStats {
    pub kes_public_key: String,
    pub vrf_public_key: String,
    pub total_stake: u64,
    pub rewards: Rewards,
    pub tax: TaxTypeSerde,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rewards {
    pub epoch: u32,
    #[serde(with = "ValueDef")]
    pub value_taxed: Value,
    #[serde(with = "ValueDef")]
    pub value_for_stakers: Value,
}
