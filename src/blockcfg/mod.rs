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
    Block, BlockDate, BlockId, ChainLength, Deserialize, FromStr, HasHeader, HasMessages, Header,
    LeaderSelection, Ledger, Message, MessageId, Serialize, Settings, State, Transaction,
    TransactionId,
};
pub use network_core::gossip::Gossip;

pub mod genesis_data;
pub mod mock;

use std::fmt::{Debug, Display};

pub trait BlockConfig {
    type Block: Block<Id = Self::BlockHash, Date = Self::BlockDate, ChainLength = Self::ChainLength>
        + HasHeader<Header = Self::BlockHeader>
        + HasMessages<Message = Self::Message>
        + Send;
    type BlockDate: BlockDate + Display + FromStr;
    type ChainLength: ChainLength;
    type BlockHash: BlockId + Display + Send;
    type BlockHeader: Header<
            Id = Self::BlockHash,
            Date = Self::BlockDate,
            ChainLength = Self::ChainLength,
        > + Clone
        + Send
        + Sync
        + Debug;
    type Transaction: Transaction + Send + Clone;
    type TransactionId: TransactionId + Serialize + Deserialize;
    type Message: Message<Id = Self::MessageId> + Send + Clone;
    type MessageId: MessageId + Send;
    type GenesisData;

    type State: State<Header = Self::BlockHeader, Content = Self::Message>
        + Settings<Block = Self::Block>;
    type Leadership: LeaderSelection<State = Self::State, Block = Self::Block>;

    type Gossip: Gossip + Clone + Send + Sync + Debug;

    type NodeSigningKey;

    fn make_block(
        secret_key: &Self::NodeSigningKey,
        block_date: Self::BlockDate,
        chain_length: Self::ChainLength,
        parent_id: Self::BlockHash,
        messages: Vec<Self::Message>,
    ) -> Self::Block;
}
