use crate::time::{SecondsSinceUnixEpoch, SystemTime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PeerStats {
    pub node_id: String,
    pub addr: Option<SocketAddr>,
    pub established_at: SystemTime,
    pub last_block_received: Option<SystemTime>,
    pub last_fragment_received: Option<SystemTime>,
    pub last_gossip_received: Option<SystemTime>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerRecord {
    pub profile: Profile,
    pub record: Record,
    pub logs: Logs,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub info: Info,
    pub subscriptions: Vec<Subscription>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Info {
    pub address: String,
    pub id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subscription {
    pub interest: String,
    pub topic: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub strikes: Vec<Strike>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strike {
    pub reason: String,
    pub when: When,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct When {
    pub secs_since_epoch: SecondsSinceUnixEpoch,
    pub nanos_since_epoch: u128,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Logs {
    pub creation_time: When,
    pub last_gossip: When,
    pub last_update: When,
    pub quarantined: Option<When>,
    pub last_use_of: HashMap<String, When>,
}
