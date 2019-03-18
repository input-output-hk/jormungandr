//! The global ledger/update/delegation states
//!

use crate::{
    block::{Block, Header, Message},
    leadership,
    ledger::{self, Ledger},
    setting,
    stake::DelegationState,
};
use chain_core::property::{self, Header as _};

#[derive(Clone)]
pub struct State {
    pub(crate) ledger: Ledger,
    pub(crate) settings: setting::Settings,
    pub(crate) delegation: DelegationState,
}

#[derive(Debug)]
pub enum Error {
    LedgerError(ledger::Error),
    Delegation(leadership::Error),
}
impl From<ledger::Error> for Error {
    fn from(e: ledger::Error) -> Self {
        Error::LedgerError(e)
    }
}
impl From<leadership::Error> for Error {
    fn from(e: leadership::Error) -> Self {
        Error::Delegation(e)
    }
}

impl property::State for State {
    type Error = Error;
    type Header = Header;
    type Content = Message;

    fn apply<I>(&self, header: &Self::Header, contents: I) -> Result<Self, Self::Error>
    where
        I: IntoIterator<Item = Self::Content>,
    {
        let mut new_ledger = self.ledger.clone();
        let mut new_delegation = self.delegation.clone();
        let mut new_settings = self.settings.clone();
        new_settings.last_block_id = header.id();
        new_settings.last_block_date = header.date();
        for content in contents {
            match content {
                Message::Transaction(signed_transaction) => {
                    new_ledger = new_ledger.apply_transaction(&signed_transaction)?;
                }
                Message::Update(update_proposal) => {
                    new_settings = new_settings.apply(update_proposal);
                }
                content => {
                    new_delegation = new_delegation.apply(&content)?;
                }
            }
        }
        Ok(State {
            ledger: new_ledger,
            settings: new_settings,
            delegation: new_delegation,
        })
    }

    pub fn new() -> Self {
        State {
            ledger: Ledger::new(),
            settings: setting::Settings::new(),
            delegation: DelegationState::new(Vec::new(), std::collections::HashMap::new()),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Delegation(error) => error.fmt(f),
            Error::LedgerError(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Delegation(error) => error.source(),
            Error::LedgerError(error) => error.source(),
        }
    }
}
