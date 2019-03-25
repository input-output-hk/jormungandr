//! The global ledger/update/delegation states
//!

use crate::{
    block::{Header, Message},
    leadership,
    ledger::{self, Ledger, LedgerParameters},
    setting,
    stake::{DelegationError, DelegationState},
    utxo,
};
use chain_addr::Address;

#[derive(Clone)]
pub struct State {
    pub(crate) ledger: Ledger,
    pub(crate) settings: setting::Settings,
    pub(crate) delegation: DelegationState,
    pub(crate) dyn_params: LedgerParameters,
}

#[derive(Debug)]
pub enum Error {
    LedgerError(ledger::Error),
    Leadership(leadership::Error),
    Delegation(DelegationError),
}
impl From<ledger::Error> for Error {
    fn from(e: ledger::Error) -> Self {
        Error::LedgerError(e)
    }
}
impl From<DelegationError> for Error {
    fn from(e: DelegationError) -> Self {
        Error::Delegation(e)
    }
}
impl From<leadership::Error> for Error {
    fn from(e: leadership::Error) -> Self {
        Error::Leadership(e)
    }
}

impl State {
    pub fn new(ledger: Ledger, dyn_params: LedgerParameters) -> Self {
        State {
            ledger: ledger,
            settings: setting::Settings::new(),
            delegation: DelegationState::new(),
            dyn_params: dyn_params,
        }
    }

    pub fn utxos<'a>(&'a self) -> utxo::Iter<'a, Address> {
        self.ledger.utxos.iter()
    }

    pub fn apply_block(
        &self,
        ledger_params: &LedgerParameters,
        contents: &[Message],
    ) -> Result<Self, Error> {
        let mut new_ledger = self.ledger.clone();
        let mut new_delegation = self.delegation.clone();
        let mut new_settings = self.settings.clone();

        for content in contents {
            match content {
                Message::OldUtxoDeclaration(_) => unimplemented!(),
                Message::Transaction(authenticated_tx) => {
                    new_ledger = new_ledger.apply_transaction(&authenticated_tx, &ledger_params)?;
                }
                Message::Update(update_proposal) => {
                    new_settings = new_settings.apply(update_proposal.clone());
                }
                Message::Certificate(authenticated_cert_tx) => {
                    new_ledger =
                        new_ledger.apply_transaction(authenticated_cert_tx, &ledger_params)?;
                    new_delegation =
                        new_delegation.apply(&authenticated_cert_tx.transaction.extra)?;
                }
            }
        }
        Ok(State {
            ledger: new_ledger,
            settings: new_settings,
            delegation: new_delegation,
            dyn_params: self.dyn_params.clone(),
        })
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Delegation(error) => error.fmt(f),
            Error::LedgerError(error) => error.fmt(f),
            Error::Leadership(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Delegation(error) => error.source(),
            Error::LedgerError(error) => error.source(),
            Error::Leadership(error) => error.source(),
        }
    }
}
