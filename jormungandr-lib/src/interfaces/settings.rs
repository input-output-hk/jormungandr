use crate::{interfaces::LinearFeeDef, time::SystemTime};
use chain_impl_mockchain::fee::LinearFee;
use serde::{Deserialize, Serialize};

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
    pub slot_duration: Option<u64>,
    pub slots_per_epoch: Option<u32>,
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
    }
}
