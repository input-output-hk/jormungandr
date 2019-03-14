//! The global ledger/update/delegation states
//!

use crate::block::{BlockContents, Message};
use crate::ledger::Ledger;
use crate::{ledger, setting};

pub struct State {
    pub(crate) ledger: Ledger,
    pub(crate) settings: setting::Settings,
}

pub enum Error {
    LedgerError(ledger::Error),
}
impl From<ledger::Error> for Error {
    fn from(e: ledger::Error) -> Self {
        Error::LedgerError(e)
    }
}

impl State {
    pub fn apply(&self, contents: BlockContents) -> Result<State, Error> {
        // for now we just clone ledger, since leadership is still inside the state.
        let mut new_ledger = self.ledger.clone();
        for content in contents.iter() {
            match content {
                Message::Transaction(signed_transaction) => {
                    let ledger = new_ledger.apply_transaction(signed_transaction)?;
                    new_ledger = ledger;
                }
                Message::Update(_update_proposal) => {}
                _ => {}
            }
        }
        Ok(State {
            ledger: new_ledger,
            settings: self.settings.clone(),
        })
    }
}

impl State {
    pub fn new() -> Self {
        State {
            ledger: Ledger::new(),
            settings: setting::Settings::new(),
        }
    }
}
