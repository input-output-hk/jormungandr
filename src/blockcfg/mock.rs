//! This module defines some basic type to try to mock the blockchain
//! and be able to run simpler tests.
//!

use crate::blockcfg::{genesis_data::GenesisData, BlockConfig};
use chain_addr::Address;
use chain_impl_mockchain::*;
use network::p2p_topology as p2p;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mockchain;
impl BlockConfig for Mockchain {
    type Block = block::Block;
    type BlockDate = block::BlockDate;
    type BlockHash = key::Hash;
    type BlockHeader = block::Header;
    type Transaction = transaction::SignedTransaction<Address>;
    type TransactionId = transaction::TransactionId;
    type Message = block::Message;
    type MessageId = block::message::MessageId;
    type GenesisData = GenesisData;
    type State = state::State;
    type Leadership = leadership::Leadership;

    type NodeSigningKey = leadership::Leader;

    type Gossip = p2p::Gossip;

    fn make_block(
        secret_key: &Self::NodeSigningKey,
        block_date: Self::BlockDate,
        parent_id: Self::BlockHash,
        messages: Vec<Self::Message>,
    ) -> Self::Block {
        let mut builder = block::BlockBuilder::new();
        builder.messages(messages)
            .date(block_date)
            .parent(parent_id)
            // TODO: .chain_length(chain_length)
            ;

        match secret_key {
            leadership::Leader::None => builder.make_genesis_block(),
            leadership::Leader::BftLeader(bft) => builder.make_bft_block(&bft),
            leadership::Leader::GenesisPraos(_, _, _) => unimplemented!(),
        }
    }
}

impl network_grpc::client::ProtocolConfig for Mockchain {
    type Block = block::Block;
    type BlockDate = block::BlockDate;
    type BlockId = key::Hash;
    type Header = block::Header;
    type Gossip = p2p::Gossip;
}
