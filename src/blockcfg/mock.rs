//! This module defines some basic type to try to mock the blockchain
//! and be able to run simpler tests.
//!

use crate::blockcfg::{genesis_data::GenesisData, BlockConfig};
use chain_impl_mockchain::*;
use network::p2p_topology as p2p;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mockchain;
impl BlockConfig for Mockchain {
    type Block = block::Block;
    type BlockDate = block::BlockDate;
    type BlockHash = key::Hash;
    type BlockHeader = block::Header;
    type Message = block::Message;
    type MessageId = block::message::MessageId;
    type GenesisData = GenesisData;
    type State = state::State;

    type NodeSigningKey = leadership::Leader;

    type Gossip = p2p::Gossip;

/*
    fn make_block(
        _secret_key: &Self::NodeSigningKey,
        settings: &Self::Settings,
        _ledger: &Self::Ledger,
        block_date: Self::BlockDate,
        transactions: Vec<Self::Transaction>,
    ) -> Self::Block {
        use chain_core::property::Settings;

        let content = block::BlockContents::new(
            transactions
                .into_iter()
                .map(block::Message::Transaction)
                .collect(),
        );

        let (content_hash, content_size) = content.compute_hash_size();

        let common = block::Common {
            block_version: block::BLOCK_VERSION_CONSENSUS_NONE,
            block_date: block_date,
            block_content_size: content_size as u32,
            block_content_hash: content_hash,
            block_parent_hash: settings.tip(),
        };

        block::Block::new(content, common, &mut leadership::Leader::None)
    }
    */
}

impl network_grpc::client::ProtocolConfig for Mockchain {
    type Block = block::Block;
    type BlockDate = block::BlockDate;
    type BlockId = key::Hash;
    type Header = block::Header;
    type Gossip = p2p::Gossip;
}
