#![allow(dead_code)]

extern crate serde_derive;
use self::serde_derive::{Deserialize, Serialize};
use crate::common::configuration::genesis_model::LinearFees;
use jormungandr_lib::time::SystemTime;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub block0_hash: String,
    pub block0_time: SystemTime,
    pub curr_slot_start_time: SystemTime,
    pub consensus_version: String,
    pub fees: LinearFees,
    pub max_txs_per_block: u8,
}
