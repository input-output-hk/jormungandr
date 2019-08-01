#![allow(dead_code)]

extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};
use crate::common::configuration::genesis_model::LinearFees;
use jormungandr_lib::time::SystemTime;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub block0_hash: String,
    pub block0_time: SystemTime,
    pub curr_slot_start_time: SystemTime,
    pub consensus_version: String,
    pub fees: LinearFees,
    pub max_txs_per_block: u8,
}

impl PartialEq<Settings> for Settings {
    fn eq(&self, other: &Settings) -> bool {
        self.block0_hash == other.block0_hash
            && self.block0_time == other.block0_time
            && self.consensus_version == other.consensus_version
            && self.fees == other.fees
            && self.max_txs_per_block == other.max_txs_per_block
    }
}
