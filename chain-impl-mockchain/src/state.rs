//! The global ledger/update/delegation states
//!

use crate::{
    leadership,
    ledger::{self, Ledger, LedgerParameters},
    utxo,
};
use chain_addr::Address;

#[derive(Clone)]
pub struct State {
    pub(crate) ledger: Ledger,
    pub(crate) dyn_params: LedgerParameters,
}

#[derive(Debug)]
pub enum Error {
    LedgerError(ledger::Error),
    Leadership(leadership::Error),
}
impl From<ledger::Error> for Error {
    fn from(e: ledger::Error) -> Self {
        Error::LedgerError(e)
    }
}
impl From<leadership::Error> for Error {
    fn from(e: leadership::Error) -> Self {
        Error::Leadership(e)
    }
}

impl State {
    pub fn new(ledger: Ledger, dyn_params: LedgerParameters) -> Self {
        // FIXME here we want to extract all the parameters we need for running.
        // we don't want to store the ledger at all here.
        State {
            ledger: ledger,
            dyn_params: dyn_params,
        }
    }

    pub fn utxos<'a>(&'a self) -> utxo::Iter<'a, Address> {
        self.ledger.utxos.iter()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::LedgerError(error) => error.fmt(f),
            Error::Leadership(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::LedgerError(error) => error.source(),
            Error::Leadership(error) => error.source(),
        }
    }
}
