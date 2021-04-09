use std::convert::TryInto;

pub mod comm;
mod gossip;
pub mod layers;
mod policy;

mod topology;

pub use self::gossip::{Gossip, Gossips, Peer, Peers};
pub use self::topology::{P2pTopology, ProfileInfo};
use chain_crypto::Ed25519;
use jormungandr_lib::crypto::key::SigningKey;
pub use poldercast::Profile;
pub use policy::{PolicyConfig, Quarantine};

pub type Address = std::net::SocketAddr;
pub type NodeId = keynesis::key::ed25519::PublicKey;
pub type SecretKey = keynesis::key::ed25519::SecretKey;

pub fn secret_key_into_keynesis(key: SigningKey<Ed25519>) -> SecretKey {
    let key_bytes = key.into_secret_key().leak_secret();
    key_bytes.as_ref().try_into().unwrap()
}

pub fn identifier_into_keynesis(id: jormungandr_lib::interfaces::NodeId) -> NodeId {
    let id_bytes = id.into_public_key().inner();
    id_bytes.as_ref().try_into().unwrap()
}

/**
# topics definition for p2p interest subscriptions
*/
pub mod topic {
    use poldercast::Topic;

    pub const MESSAGES: Topic = Topic::new([0; 32]);
    pub const BLOCKS: Topic = {
        let mut array = [0; 32];
        array[31] = 1;
        Topic::new(array)
    };
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
