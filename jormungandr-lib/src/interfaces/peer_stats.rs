use crate::time::SystemTime;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PeerStats {
    pub addr: Option<SocketAddr>,
    pub established_at: SystemTime,
    pub last_block_received: Option<SystemTime>,
    pub last_fragment_received: Option<SystemTime>,
    pub last_gossip_received: Option<SystemTime>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerRecord {
    pub id: String,
    pub address: String,
    pub last_update: SystemTime,
    pub quarantined: Option<SystemTime>,
    pub subscriptions: Vec<Subscription>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subscription {
    pub interest: u32,
    pub topic: String,
}
