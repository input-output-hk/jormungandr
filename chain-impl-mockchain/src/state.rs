//! The global ledger/update/delegation states
//!

use crate::{
    block::{Header, Message},
    leadership,
    ledger::{self, Ledger, LedgerParameters, LedgerStaticParameters},
    setting,
    stake::{DelegationError, DelegationState},
    utxo,
};
use chain_addr::{Address, Discrimination};

#[derive(Clone)]
pub struct State {
    pub(crate) ledger: Ledger,
    pub(crate) settings: setting::Settings,
    pub(crate) delegation: DelegationState,
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
    pub fn new(address_discrimination: Discrimination) -> Self {
        State {
            ledger: Ledger::new(),
            settings: setting::Settings::new(address_discrimination),
            delegation: DelegationState::new(),
        }
    }

    pub fn utxos<'a>(&'a self) -> utxo::Iter<'a, Address> {
        self.ledger.utxos.iter()
    }

    pub fn apply_block(&self, contents: &[Message]) -> Result<Self, Error> {
        let mut new_ledger = self.ledger.clone();
        let mut new_delegation = self.delegation.clone();
        let mut new_settings = self.settings.clone();

        let static_params = LedgerStaticParameters {
            allow_account_creation: new_settings.allow_account_creation(),
            discrimination: *new_settings.address_discrimination(),
        };
        let dyn_params = LedgerParameters {
            fees: new_settings.linear_fees(),
        };

        for content in contents {
            match content {
                Message::OldUtxoDeclaration(_) => unimplemented!(),
                Message::Transaction(authenticated_tx) => {
                    new_ledger = new_ledger.apply_transaction(
                        &authenticated_tx,
                        &static_params,
                        &dyn_params,
                    )?;
                }
                Message::Update(update_proposal) => {
                    new_settings = new_settings.apply(update_proposal.clone());
                }
                Message::Certificate(authenticated_cert_tx) => {
                    new_ledger = new_ledger.apply_transaction(
                        authenticated_cert_tx,
                        &static_params,
                        &dyn_params,
                    )?;
                    new_delegation =
                        new_delegation.apply(&authenticated_cert_tx.transaction.extra)?;
                }
            }
        }
        Ok(State {
            ledger: new_ledger,
            settings: new_settings,
            delegation: new_delegation,
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
