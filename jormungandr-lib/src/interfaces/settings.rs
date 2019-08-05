use crate::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub block0_hash: String,
    pub block0_time: SystemTime,
    pub curr_slot_start_time: Option<SystemTime>,
    pub consensus_version: String,
    pub fees: SettingsFeesDto,
    pub max_txs_per_block: u32,
}

impl PartialEq<SettingsDto> for SettingsDto {
    fn eq(&self, other: &SettingsDto) -> bool {
        self.block0_hash == other.block0_hash
            && self.block0_time == other.block0_time
            && self.consensus_version == other.consensus_version
            && self.fees == other.fees
            && self.max_txs_per_block == other.max_txs_per_block
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsFeesDto {
    pub constant: u64,
    pub coefficient: u64,
    pub certificate: u64,
}
