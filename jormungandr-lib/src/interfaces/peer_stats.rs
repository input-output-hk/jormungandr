use crate::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PeerStats {
    pub node_id: String,
    pub addr: String,
    pub established_at: SystemTime,
    pub last_block_received: SystemTime,
    pub last_fragment_received: SystemTime,
    pub last_gossip_received: SystemTime,
}
