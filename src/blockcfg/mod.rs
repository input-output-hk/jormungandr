pub use network_core::gossip::Gossip;

pub mod genesis_data;

pub use chain_impl_mockchain::{
    block::{Block, Header, HeaderHash, Message, MessageId},
    ledger::Ledger,
    multiverse::MultiVerse,
    state::State,
};
