//! The global ledger/update/delegation states
//!

use crate::{
    block::{BlockContents, Message},
    leadership,
    ledger::{self, Ledger},
    setting,
    stake::DelegationState,
};

pub struct State {
    pub(crate) ledger: Ledger,
    pub(crate) settings: setting::Settings,
    pub(crate) delegation: DelegationState,
}

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

impl State {
    pub fn apply(&self, contents: BlockContents) -> Result<State, Error> {
        // for now we just clone ledger, since leadership is still inside the state.
        let mut new_ledger = self.ledger.clone();
        let mut new_delegation = self.delegation.clone();
        for content in contents.iter() {
            match content {
                Message::Transaction(signed_transaction) => {
                    let ledger = new_ledger.apply_transaction(signed_transaction)?;
                    new_ledger = ledger;
                }
                Message::Update(_update_proposal) => {}
                content => {
                    let delegation = new_delegation.apply(content)?;
                }
            }
        }
        Ok(State {
            ledger: new_ledger,
            settings: self.settings.clone(),
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
