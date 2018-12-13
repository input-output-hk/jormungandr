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

pub mod chain;
pub mod ledger;
pub mod update;
// TODO: pub mod consensus;

mod cardano;

pub use self::cardano::{Cardano};

pub trait BlockConfig {
    type Block: chain::Block<Hash = Self::BlockHash>
        + ledger::HasTransaction<Transaction = Self::Transaction>;
    type BlockHash;
    type BlockHeader;
    type Transaction: ledger::Transaction<Id = Self::TransactionId>;
    type TransactionId;
    type GenesisData;

    type Ledger: ledger::Ledger<Transaction = Self::Transaction>
        + update::Update<Block = Self::Block>;

    fn make_block(
        secret_key: &secure::NodeSecret,
        public_key: &secure::NodePublic,
        ledger: &Self::Ledger,
        block_id: <Self::Block as chain::Block>::Id,
        transactions: Vec<Self::Transaction>,
    ) -> Self::Block;
}
