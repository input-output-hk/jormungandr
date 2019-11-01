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
