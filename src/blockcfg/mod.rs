//! This module provides the different abstractions for the different
//! part of the blockchain.
//!
//! It has been split into 3 components:
//!
//! 1. chain: all the components that chains blocks together;
//! 2. ledger: the transaction model of a blockchain;
//! 3. consensus: the consensus model of the blockchain.
//!

pub mod chain;
pub mod ledger;
// TODO: pub mod consensus;

// ---------------------------------------------------------------
// below we defined what we are using at the moment in jormungandr
// for the blockchain, we might want to change this in the future
// and have a more explicit choice at the top level.
//
pub use chain::cardano::{
    Block,
    Header,
    BlockHash,
    Transaction,
    TransactionId,
    GenesisData,
};