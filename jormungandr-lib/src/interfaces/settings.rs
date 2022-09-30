use crate::{
    interfaces::{LinearFeeDef, ValueDef},
    time::SystemTime,
};
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    block::Epoch,
    fee::LinearFee,
    rewards::{CompoundingType, Limit, Parameters, Ratio, TaxType},
    value::Value,
};
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
    pub block_content_max_size: u32,
    pub epoch_stability_depth: u32,
    pub slot_duration: u64,
    pub slots_per_epoch: u32,
    #[serde(with = "TaxTypeDef")]
    pub treasury_tax: TaxType,
    #[serde(with = "ParametersDef")]
    pub reward_params: Parameters,
    #[serde(with = "DiscriminationDef")]
    pub discrimination: Discrimination,
    pub tx_max_expiry_epochs: u8,
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

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct TaxTypeSerde(#[serde(with = "TaxTypeDef")] pub TaxType);

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(remote = "Limit")]
pub enum LimitDef {
    None,
    ByStakeAbsolute(#[serde(with = "RatioDef")] Ratio),
}

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
    #[serde(with = "LimitDef")]
    pub reward_drawing_limit_max: Limit,
    pub pool_participation_capping: Option<(NonZeroU32, NonZeroU32)>,
}

#[derive(Deserialize, Serialize)]
#[serde(remote = "CompoundingType")]
pub enum CompoundingTypeDef {
    Linear,
    Halvening,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", remote = "Discrimination")]
enum DiscriminationDef {
    Test,
    Production,
}

impl PartialEq<SettingsDto> for SettingsDto {
    fn eq(&self, other: &SettingsDto) -> bool {
        self.block0_hash == other.block0_hash
            && self.block0_time == other.block0_time
            && self.consensus_version == other.consensus_version
            && self.fees == other.fees
            && self.block_content_max_size == other.block_content_max_size
            && self.epoch_stability_depth == other.epoch_stability_depth
            && self.slot_duration == other.slot_duration
            && self.slots_per_epoch == other.slots_per_epoch
            && self.treasury_tax == other.treasury_tax
            && self.reward_params == other.reward_params
    }
}
