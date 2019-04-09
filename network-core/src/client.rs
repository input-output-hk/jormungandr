//! Abstractions for the client-side network interface of a blockchain node.

pub mod block;
pub mod gossip;

use crate::gossip::NodeId;

/// Base trait for the client services that use node identifiers to
/// distinguish subscription streams.
pub trait P2pService {
    /// Network node identifier.
    type NodeId: NodeId;
}
