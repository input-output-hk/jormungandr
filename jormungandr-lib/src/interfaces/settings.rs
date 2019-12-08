use crate::{
    interfaces::{LinearFeeDef, ValueDef},
    time::SystemTime,
};
use chain_impl_mockchain::block::Epoch;
use chain_impl_mockchain::fee::LinearFee;
use chain_impl_mockchain::rewards::{CompoundingType, Parameters, Ratio, TaxType};
use chain_impl_mockchain::value::Value;
use serde::{Deserialize, Serialize};
use std::num::{NonZeroU32, NonZeroU64};

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsDto {
    pub block0_hash: String,
    pub block0_time: SystemTime,
    pub curr_slot_start_time: Option<SystemTime>,
    pub consensus_version: String,
    #[serde(with = "LinearFeeDef")]
    pub fees: LinearFee,
    pub max_txs_per_block: u32,
    pub slot_duration: u64,
    pub slots_per_epoch: u32,
    #[serde(with = "TaxTypeDef")]
    pub treasury_tax: TaxType,
    #[serde(with = "ParametersDef")]
    pub reward_params: Parameters,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "TaxType")]
pub struct TaxTypeDef {
    #[serde(with = "ValueDef")]
    pub fixed: Value,

    #[serde(with = "RatioDef")]
    pub ratio: Ratio,

    #[serde(default, rename = "max", skip_serializing_if = "Option::is_none")]
    pub max_limit: Option<NonZeroU64>,
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct TaxTypeSerde(#[serde(with = "TaxTypeDef")] pub TaxType);

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(remote = "Ratio")]
pub struct RatioDef {
    pub numerator: u64,
    pub denominator: NonZeroU64,
}

#[derive(Deserialize, Serialize)]
#[serde(remote = "Parameters", rename_all = "camelCase")]
pub struct ParametersDef {
    pub initial_value: u64,
    #[serde(with = "RatioDef")]
    pub compounding_ratio: Ratio,
    #[serde(with = "CompoundingTypeDef")]
    pub compounding_type: CompoundingType,
    pub epoch_rate: NonZeroU32,
    pub epoch_start: Epoch,
}

#[derive(Deserialize, Serialize)]
#[serde(remote = "CompoundingType")]
pub enum CompoundingTypeDef {
    Linear,
    Halvening,
}

impl PartialEq<SettingsDto> for SettingsDto {
    fn eq(&self, other: &SettingsDto) -> bool {
        self.block0_hash == other.block0_hash
            && self.block0_time == other.block0_time
            && self.consensus_version == other.consensus_version
            && self.fees == other.fees
            && self.max_txs_per_block == other.max_txs_per_block
            && self.slot_duration == other.slot_duration
            && self.slots_per_epoch == other.slots_per_epoch
            && self.treasury_tax == other.treasury_tax
            && self.reward_params == other.reward_params
    }
}
