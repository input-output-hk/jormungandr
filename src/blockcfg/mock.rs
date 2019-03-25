//! This module defines some basic type to try to mock the blockchain
//! and be able to run simpler tests.
//!

use crate::blockcfg::{genesis_data::GenesisData, BlockConfig};
use chain_addr::Address;
use chain_impl_mockchain::*;
use network::p2p_topology as p2p;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mockchain;

impl network_grpc::client::ProtocolConfig for Mockchain {
    type Block = block::Block;
    type BlockDate = block::BlockDate;
    type BlockId = key::Hash;
    type Header = block::Header;
    type Gossip = p2p::Gossip;
}
