use crate::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Stats {
    pub tx_recv_cnt: u32,
    pub block_recv_cnt: u32,
    pub uptime: u32,
    pub state: NodeState,
    pub last_block_hash: String,
    pub last_block_height: String,
    pub last_block_date: String,
    pub last_block_time: Option<SystemTime>,
    pub last_block_tx: u32,
    pub last_block_sum: u32,
    pub last_block_fees: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum NodeState {
    StartingRestServer,
    PreparingStorage,
    PreparingBlock0,
    Bootstrapping,
    StartingWorkers,
    Running,
}
