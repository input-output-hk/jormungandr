//! The global ledger/update/delegation states
//!

use crate::block::BlockContents;
use crate::{account, block, leadership, setting, utxo};
use cardano::address::Addr as OldAddress;
use chain_addr::Address;
use chain_core::property;

pub(crate) type Leadership = Box<
    dyn property::LeaderSelection<
        Update = leadership::Update,
        Block = block::Block,
        Error = leadership::Error,
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

impl State {
    pub fn apply(&self, content: BlockContents) -> State {
        unimplemented!()
    }
}

impl Ledger {
    fn new() -> Self {
        Ledger {
            utxos: utxo::Ledger::new(),
            oldutxos: utxo::Ledger::new(),
            accounts: account::Ledger::new(),
        }
    }
}

impl State {
    pub fn new() -> Self {
        State {
            ledger: Ledger::new(),
            settings: setting::Settings::new(),
            leadership: Box::new(leadership::none::NoLeadership),
        }
    }
}
