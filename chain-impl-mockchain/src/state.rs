//! The global ledger/update/delegation states
//!

use crate::block::BlockContents;
use crate::{account, leadership, setting, utxo};
use cardano::address::Addr as OldAddress;
use chain_addr::Address;
use chain_core::property;

pub(crate) type Leadership = Box<
    dyn property::LeaderSelection<
        Update = u8,
        Block = u8,
        Error = std::io::Error,
        LeaderId = leadership::PublicLeader,
    >,
>;

pub struct Ledger {
    pub(crate) utxos: utxo::Ledger<Address>,
    pub(crate) oldutxos: utxo::Ledger<OldAddress>,
    pub(crate) accounts: account::Ledger,
}

pub struct State {
    pub(crate) ledger: Ledger,
    pub(crate) settings: setting::Settings,
    pub(crate) leadership: Leadership,
}

type Error = ();

impl State {
    pub fn apply(&self, contents: BlockContents) -> Result<State, Error> {
        for content in contents.iter() {}
        unimplemented!()
    }
}
