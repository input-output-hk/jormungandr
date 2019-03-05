//! This module defines some basic type to try to mock the blockchain
//! and be able to run simpler tests.
//!

use crate::blockcfg::{genesis_data::GenesisData, BlockConfig};
use chain_core::property;
use chain_impl_mockchain::*;
use network_core::gossip::Gossip;

// Temporary solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmptyGossip(());
impl Gossip for EmptyGossip where {}

impl property::Serialize for EmptyGossip {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, _writer: W) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl property::Deserialize for EmptyGossip {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(_reader: R) -> Result<Self, Self::Error> {
        Ok(EmptyGossip(()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mockchain;
impl BlockConfig for Mockchain {
    type Block = block::SignedBlock;
    type BlockDate = block::BlockDate;
    type BlockHash = key::Hash;
    type BlockHeader = block::SignedBlockSummary;
    type Transaction = transaction::SignedTransaction;
    type TransactionId = transaction::TransactionId;
    type GenesisData = GenesisData;
    type Ledger = ledger::Ledger;
    type Settings = setting::Settings;
    type Leader = leadership::genesis::GenesisLeaderSelection;
    type Update = update::Diff;

    type NodeSigningKey = key::PrivateKey;

    type Gossip = EmptyGossip;

    fn make_block(
        secret_key: &Self::NodeSigningKey,
        settings: &Self::Settings,
        _ledger: &Self::Ledger,
        block_date: Self::BlockDate,
        transactions: Vec<Self::Transaction>,
    ) -> Self::Block {
        use chain_core::property::Settings;

        let block = block::Block {
            slot_id: block_date,
            parent_hash: settings.tip(),
            contents: transactions
                .into_iter()
                .map(|tx| block::Message::Transaction(tx))
                .collect(),
        };

        block::SignedBlock::new(block, secret_key)
    }
}
