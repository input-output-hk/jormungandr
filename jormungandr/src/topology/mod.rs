//! This module is responsible for discovering peers and
//! selecting the subset to which we propagate info
//!
use crate::network::p2p::Address;
use jormungandr_lib::{interfaces::Subscription, time::SystemTime};
use serde::{Serialize, Serializer};
use std::{
    convert::{TryFrom, TryInto},
    fmt,
    hash::{Hash, Hasher},
};

mod gossip;
pub mod layers;
mod process;
mod quarantine;
#[allow(clippy::module_inception)]
mod topology;

pub use self::{
    gossip::{Gossip, Gossips},
    process::{start, TaskData, DEFAULT_NETWORK_STUCK_INTERVAL},
    topology::{P2pTopology, View},
};
pub use quarantine::{QuarantineConfig, ReportRecords};

/**
# topics definition for p2p interest subscriptions
*/
pub mod topic {
    use poldercast::Topic;

    pub const MESSAGES: Topic = Topic::new([0; 32]);
    pub const BLOCKS: Topic = Topic::new([1; 32]);
}

/**
limits for the property::{Serialize/Deserialize} implementations
*/
pub mod limits {
    /// limit the gossip size to 512 bytes (limit per gossip).
    ///
    /// a gossip only contains the Id, the address and an array of subscriptions
    /// which should not go beyond 2 2-tuples of 64bits.
    pub const MAX_GOSSIP_SIZE: usize = 512;

    /// limit the ID size to 32 bytes. Right now the Node ID are 24 bytes but
    /// for backward compatibility keep the value to 32bytes.
    pub const MAX_ID_SIZE: u64 = 32;
}

/// Unique identifier of a node in the topology
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct NodeId(keynesis::key::ed25519::PublicKey);

impl From<jormungandr_lib::interfaces::NodeId> for NodeId {
    fn from(id: jormungandr_lib::interfaces::NodeId) -> Self {
        let id_bytes = id.as_ref().as_ref();
        NodeId(id_bytes.try_into().unwrap())
    }
}

impl From<NodeId> for jormungandr_lib::interfaces::NodeId {
    fn from(node_id: NodeId) -> jormungandr_lib::interfaces::NodeId {
        jormungandr_lib::interfaces::NodeId::from_hex(&node_id.0.to_string()).unwrap()
    }
}

impl TryFrom<&[u8]> for NodeId {
    type Error = chain_crypto::PublicKeyError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        use chain_crypto::{Ed25519, PublicKey};
        Ok(Self::from(
            PublicKey::<Ed25519>::from_binary(bytes)
                .map(jormungandr_lib::interfaces::NodeId::from)?,
        ))
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<keynesis::key::ed25519::PublicKey> for NodeId {
    fn as_ref(&self) -> &keynesis::key::ed25519::PublicKey {
        &self.0
    }
}

impl Serialize for NodeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            self.0.to_string().serialize(serializer)
        } else {
            self.0.as_ref().serialize(serializer)
        }
    }
}

/// This represents a peer and its public key used for
/// identification in the topology.
pub type Peer = Gossip;

#[derive(Eq, Clone, Serialize, Debug)]
pub struct PeerInfo {
    pub id: NodeId,
    pub address: Address,
    pub last_update: SystemTime,
    pub quarantined: Option<SystemTime>,
    pub subscriptions: Vec<Subscription>,
}

impl PartialEq for PeerInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.address == other.address
    }
}

impl Hash for PeerInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.address.hash(state);
    }
}

impl From<Peer> for PeerInfo {
    fn from(other: Peer) -> Self {
        let other: poldercast::Gossip = other.into();
        Self {
            id: NodeId(other.id()),
            address: other.address(),
            last_update: other.time().to_system_time().into(),
            quarantined: None,
            subscriptions: other
                .subscriptions()
                .iter()
                .map(|s| Subscription {
                    topic: s.topic().to_string(),
                    interest: s
                        .interest_level()
                        .priority_score(poldercast::InterestLevel::ZERO)
                        as u32,
                })
                .collect(),
        }
    }
}
