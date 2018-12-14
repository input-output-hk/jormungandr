//! This module provides the different abstractions for the different
//! part of the blockchain.
//!
//! It has been split into 3 components:
//!
//! 1. chain: all the components that chains blocks together;
//! 2. ledger: the transaction model of a blockchain;
//! 3. consensus: the consensus model of the blockchain.
//!

use crate::secure;

pub mod property;

pub mod cardano;
#[cfg(test)]
pub mod mock;

pub trait BlockConfig {
    type Block: property::Block<Hash = Self::BlockHash>
        + property::HasTransaction<Transaction = Self::Transaction>;
    type BlockHash;
    type BlockHeader;
    type Transaction: property::Transaction<Id = Self::TransactionId>;
    type TransactionId;
    type GenesisData;

    type Ledger: property::Ledger<Transaction = Self::Transaction>
        + property::Update<Block = Self::Block>;

    fn make_block(
        secret_key: &secure::NodeSecret,
        public_key: &secure::NodePublic,
        ledger: &Self::Ledger,
        block_id: <Self::Block as property::Block>::Id,
        transactions: Vec<Self::Transaction>,
    ) -> Self::Block;
}
