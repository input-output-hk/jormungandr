//! Abstractions for the server-side network interface of a blockchain node.

mod block;
mod fragment;
mod gossip;

pub mod request_stream;

pub use block::BlockService;
pub use fragment::FragmentService;
pub use gossip::GossipService;

use crate::gossip::NodeId;

/// Interface to application logic of the blockchain node server.
///
/// An implementation of a blockchain node implements this trait to
/// serve the network protocols using node's subsystems such as
/// block storage and transaction engine.
///
/// A `Node` implementation is expected to be stateless, that is,
/// there is no particular association between client peers and instances
/// of the implementing type, and conversely, multiple instances can be
/// created to serve different requests from a single client.
pub trait Node {
    /// The implementation of the block service.
    type BlockService: BlockService;

    /// The implementation of the content service.
    type FragmentService: FragmentService;

    /// The implementation of the gossip service.
    type GossipService: GossipService;

    /// Instantiates the block service,
    /// if supported by this node.
    fn block_service(&mut self) -> Option<&mut Self::BlockService>;

    /// Instantiates the fragment service,
    /// if supported by this node.
    fn fragment_service(&mut self) -> Option<&mut Self::FragmentService>;

    /// Instantiates the gossip service,
    /// if supported by this node.
    fn gossip_service(&mut self) -> Option<&mut Self::GossipService>;
}

/// Base trait for the services that use node identifiers to
/// distinguish subscription streams.
pub trait P2pService {
    /// Network node identifier.
    type NodeId: NodeId + Send + 'static;

    /// Returns the identifier of this node.
    fn node_id(&self) -> Self::NodeId;
}
