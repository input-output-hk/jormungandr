//! This module is responsible for discovering peers and
//! selecting the subset to which we propagate info
//!
use crate::network::p2p::Address;
use jormungandr_lib::interfaces::Subscription;
use jormungandr_lib::time::SystemTime;
use poldercast::Profile;
use serde::Serialize;
use serde::Serializer;
use std::convert::TryInto;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

mod gossip;
pub mod layers;
mod process;
mod quarantine;
#[allow(clippy::module_inception)]
mod topology;

pub use self::gossip::{Gossip, Gossips};
pub use self::process::{start, TaskData};
pub use self::topology::{P2pTopology, View};
pub use quarantine::{Quarantine, QuarantineConfig};

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
    pub const MAX_GOSSIP_SIZE: u64 = 512;

    /// limit the ID size to 32 bytes. Right now the Node ID are 24 bytes but
    /// for backward compatibility keep the value to 32bytes.
    pub const MAX_ID_SIZE: u64 = 32;
}

/// Unique identifier of a node in the topology
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Peer(Gossip);

impl Peer {
    pub fn address(&self) -> Address {
        self.0.address()
    }

    pub fn id(&self) -> NodeId {
        self.0.id()
    }
}

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

impl From<&Arc<Profile>> for PeerInfo {
    fn from(other: &Arc<Profile>) -> Self {
        Self {
            id: NodeId(other.id()),
            address: other.address(),
            last_update: other.last_update().to_system_time().into(),
            quarantined: None,
            subscriptions: other
                .subscriptions()
                .iter()
                .map(|s| Subscription {
                    topic: s.topic().to_string(),
                    interest: format!("{:?}", s.interest_level()).parse::<u32>().unwrap(),
                })
                .collect(),
        }
    }
}

impl From<Gossip> for Peer {
    fn from(gossip: Gossip) -> Self {
        Self(gossip)
    }
}

impl From<poldercast::Gossip> for Peer {
    fn from(gossip: poldercast::Gossip) -> Self {
        Self(Gossip::from(gossip))
    }
}

impl From<Peer> for Gossip {
    fn from(peer: Peer) -> Self {
        peer.0
    }
}
