//! Abstractions for the server-side network interface of a blockchain node.

pub mod block;
pub mod content;
pub mod gossip;

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
    type BlockService: block::BlockService;

    /// The implementation of the content service.
    type ContentService: content::ContentService;

    /// The implementation of the gossip service.
    type GossipService: gossip::GossipService;

    /// Instantiates the block service,
    /// if supported by this node.
    fn block_service(&mut self) -> Option<&mut Self::BlockService>;

    /// Instantiates the content service,
    /// if supported by this node.
    fn content_service(&mut self) -> Option<&mut Self::ContentService>;

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
