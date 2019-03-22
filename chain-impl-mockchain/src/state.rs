//! The global ledger/update/delegation states
//!

use crate::{
    block::{Block, Header, Message},
    leadership,
    ledger::{self, Ledger, LedgerParameters, LedgerStaticParameters},
    setting,
    stake::{DelegationError, DelegationState},
    utxo,
};
use chain_addr::{Address, Discrimination};
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

impl property::State for State {
    type Error = Error;
    type Header = Header;
    type Content = Message;

    fn apply_block<'a, I>(&self, header: &Self::Header, contents: I) -> Result<Self, Self::Error>
    where
        I: IntoIterator<Item = &'a Self::Content>,
        Self::Content: 'a,
    {
        let mut new_state = self.apply_contents(contents)?;
        new_state.settings.last_block_id = header.id();
        new_state.settings.last_block_date = header.date();
        new_state.settings.chain_length = header.common.chain_length;
        Ok(new_state)
    }

    fn apply_contents<'a, I>(&self, contents: I) -> Result<Self, Self::Error>
    where
        I: IntoIterator<Item = &'a Self::Content>,
        Self::Content: 'a,
    {
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
                Message::Transaction(signed_transaction) => {
                    new_ledger = new_ledger.apply_transaction(
                        signed_transaction,
                        &static_params,
                        &dyn_params,
                    )?;
                }
                Message::Update(update_proposal) => {
                    new_settings = new_settings.apply(update_proposal.clone());
                }
                content => {
                    new_delegation = new_delegation.apply(content)?;
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

impl property::Settings for State {
    type Block = Block;

    fn tip(&self) -> <Self::Block as property::Block>::Id {
        self.settings.tip()
    }
    fn max_number_of_transactions_per_block(&self) -> u32 {
        self.settings.max_number_of_transactions_per_block()
    }
    fn block_version(&self) -> <Self::Block as property::Block>::Version {
        self.settings.block_version()
    }
    fn chain_length(&self) -> <Self::Block as property::Block>::ChainLength {
        self.settings.chain_length()
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
