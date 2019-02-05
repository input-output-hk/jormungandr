//! This module defines some basic type to try to mock the blockchain
//! and be able to run simpler tests.
//!

use crate::blockcfg::{genesis_data::GenesisData, BlockConfig};
use chain_impl_mockchain::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mockchain;
impl BlockConfig for Mockchain {
    type Block = block::SignedBlock;
    type BlockDate = block::BlockDate;
    type BlockHash = key::Hash;
    type BlockHeader = ();
    type Transaction = transaction::SignedTransaction;
    type TransactionId = transaction::TransactionId;
    type GenesisData = GenesisData;
    type Ledger = ledger::Ledger;
    type Settings = setting::Settings;
    type Leader = leadership::LeaderSelection;
    type Update = update::Diff;

    type NodeSigningKey = key::PrivateKey;

    fn make_block(
        secret_key: &Self::NodeSigningKey,
        settings: &Self::Settings,
        ledger: &Self::Ledger,
        block_date: Self::BlockDate,
        transactions: Vec<Self::Transaction>,
    ) -> Self::Block {
        use chain_core::property::Settings;

        let block = block::Block {
            slot_id: block_date,
            parent_hash: settings.tip(),
            transactions: transactions,
        };

        block::SignedBlock::new(block, secret_key)
    }
}

pub fn xpub_to_public(xpub: &cardano::hdwallet::XPub) -> key::PublicKey {
    let mut bytes = [0; 32];
    bytes.copy_from_slice(xpub.as_ref());
    key::PublicKey::from_bytes(bytes)
}
