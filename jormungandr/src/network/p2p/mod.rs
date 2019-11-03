pub mod comm;
mod gossip;
mod id;
mod node;
mod topology;

pub use self::gossip::{Gossip, Gossips};
pub use self::id::Id;
pub use self::node::Node;
pub use self::topology::P2pTopology;

/**
# topics definition for p2p interest subscriptions
*/
pub mod topic {
    use poldercast::Topic;

    pub const MESSAGES: Topic = Topic::new(0u32);
    pub const BLOCKS: Topic = Topic::new(1u32);
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
