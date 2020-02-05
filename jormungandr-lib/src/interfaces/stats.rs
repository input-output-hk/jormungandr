use crate::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Stats {
    pub version: String,
    pub node_id: Option<String>,
    pub peer_total_cnt: Option<u32>,
    pub peer_available_cnt: Option<u32>,
    pub peer_quarantined_cnt: Option<u32>,
    pub peer_unreachable_cnt: Option<u32>,
    pub tx_recv_cnt: Option<u32>,
    pub block_recv_cnt: Option<u32>,
    pub uptime: Option<u32>,
    pub state: NodeState,
    pub last_block_hash: Option<String>,
    pub last_block_height: Option<String>,
    pub last_block_date: Option<String>,
    pub last_block_time: Option<SystemTime>,
    pub last_received_block_time: Option<SystemTime>,
    pub last_block_tx: Option<u32>,
    pub last_block_sum: Option<u32>,
    pub last_block_fees: Option<u32>,
    pub last_block_content_size: Option<u32>,
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
