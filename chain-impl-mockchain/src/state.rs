//! The global ledger/update/delegation states
//!

use crate::{account, leadership, ledger, setting};
use chain_core::property;

pub(crate) type UTxOLedger = ledger::Ledger;
pub(crate) type AccountLedger = account::Ledger;
pub(crate) type Settings = setting::Settings;
pub(crate) type Leadership =
    Box<dyn property::LeaderSelection<Update = u8, Block = u8, Error = std::io::Error, LeaderId = leadership::PublicLeader>>;

pub struct State {
    pub(crate) utxos: UTxOLedger,
    pub(crate) accounts: AccountLedger,
    pub(crate) settings: Settings,
    pub(crate) leadership: Leadership,
}
