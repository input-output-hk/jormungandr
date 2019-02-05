//! This module provides the different abstractions for the different
//! part of the blockchain.
//!
//! It has been split into 3 components:
//!
//! 1. chain: all the components that chains blocks together;
//! 2. ledger: the transaction model of a blockchain;
//! 3. consensus: the consensus model of the blockchain.
//!

pub use chain_core::property::{
    Block, BlockDate, BlockId, Deserialize, HasTransaction, Header, LeaderSelection, Ledger,
    Serialize, Settings, Transaction, TransactionId, Update,
};

pub mod genesis_data;
pub mod mock;

pub trait BlockConfig {
    type Block: Block<Id = Self::BlockHash, Date = Self::BlockDate>
        + HasTransaction<Transaction = Self::Transaction>;
    type BlockDate: BlockDate;
    type BlockHash: BlockId;
    type BlockHeader: Block<Id = Self::BlockHash, Date = Self::BlockDate>;
    type Transaction: Transaction<Id = Self::TransactionId>;
    type TransactionId: TransactionId;
    type GenesisData;

    type Ledger: Ledger<Transaction = Self::Transaction>;
    type Settings: Settings<Block = Self::Block>;
    type Leader: LeaderSelection<Block = Self::Block>;
    type Update: Update;

    type NodeSigningKey;

    fn make_block(
        secret_key: &Self::NodeSigningKey,
        settings: &Self::Settings,
        ledger: &Self::Ledger,
        block_date: <Self::Block as Block>::Date,
        transactions: Vec<Self::Transaction>,
    ) -> Self::Block;
}
