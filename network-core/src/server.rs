//! Abstractions for the server-side network interface of a blockchain node.

pub mod block;
pub mod gossip;
pub mod transaction;

/// Interface to application logic of the blockchain node server.
///
/// An implementation of a blockchain node implements this trait to
/// serve the network protocols using node's subsystems such as
/// block storage and transaction engine.
pub trait Node {
    /// The implementation of the block service.
    type BlockService: block::BlockService;

    /// The implementation of the transaction service.
    type TransactionService: transaction::TransactionService;

    /// The implementation of the gossip service.
    type GossipService: gossip::GossipService;

    /// Instantiates the block service,
    /// if supported by this node.
    fn block_service(&self) -> Option<Self::BlockService>;

    /// Instantiates the transaction service,
    /// if supported by this node.
    fn transaction_service(&self) -> Option<Self::TransactionService>;

    /// Instantiates the gossip service,
    /// if supported by this node.
    fn gossip_service(&self) -> Option<Self::GossipService>;
}
