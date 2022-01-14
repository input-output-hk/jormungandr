mod fragment_log;

use crate::jormungandr::JormungandrRest;
pub use fragment_log::{assert_accepted_rejected, assert_bad_request, FragmentLogVerifier};
use jormungandr_lib::interfaces::{AccountState, Address, Value};

pub struct JormungandrStateVerifier {
    rest: JormungandrRest,
    snapshot_before: Option<StateSnapshot>,
}

impl JormungandrStateVerifier {
    pub fn new(rest: JormungandrRest) -> Self {
        Self {
            rest,
            snapshot_before: None,
        }
    }

    pub fn fragment_logs(&self) -> FragmentLogVerifier {
        FragmentLogVerifier::new(self.rest.clone())
    }

    pub fn record_address_state(mut self, addresses: Vec<&Address>) -> Self {
        self.snapshot_before = Some(StateSnapshot::new(
            addresses
                .iter()
                .map(|w| {
                    (
                        w.to_string(),
                        self.rest
                            .account_state(&as_identifier(w))
                            .expect("cannot rerieve account state"),
                    )
                })
                .collect(),
        ));
        self
    }

    pub fn value_moved_between_addresses(
        &self,
        from: &Address,
        to: &Address,
        value: Value,
    ) -> Result<(), StateVerifierError> {
        self.address_lost_value(from, value)?;
        self.address_gained_value(to, value)?;
        Ok(())
    }

    pub fn address_lost_value(
        &self,
        address: &Address,
        value: Value,
    ) -> Result<(), StateVerifierError> {
        let snapshot = self
            .snapshot_before
            .as_ref()
            .ok_or(StateVerifierError::NoSnapshot)?;
        let expected = snapshot.value_for(address)?;
        let actual = self
            .rest
            .account_state(&as_identifier(address))?
            .value()
            .checked_add(value)?;
        assert_eq!(
            expected, actual,
            "No value was deducted friom account: {} vs {}",
            expected, actual
        );
        Ok(())
    }

    pub fn no_changes(&self, addresses: Vec<&Address>) -> Result<(), StateVerifierError> {
        for address in addresses {
            self.address_has_the_same_value(address)?;
        }
        Ok(())
    }

    pub fn address_has_the_same_value(&self, address: &Address) -> Result<(), StateVerifierError> {
        let snapshot = self
            .snapshot_before
            .as_ref()
            .ok_or(StateVerifierError::NoSnapshot)?;
        let expected = snapshot.value_for(address)?;
        let actual = *self.rest.account_state(&as_identifier(address))?.value();
        assert_eq!(
            expected, actual,
            "value changed for account {:?}: {} vs {}",
            address, expected, actual
        );
        Ok(())
    }

    pub fn address_gained_value(
        &self,
        address: &Address,
        value: Value,
    ) -> Result<(), StateVerifierError> {
        let snapshot = self
            .snapshot_before
            .as_ref()
            .ok_or(StateVerifierError::NoSnapshot)?;
        let expected = snapshot.value_for(address)?.checked_add(value)?;
        let actual = *self.rest.account_state(&as_identifier(address))?.value();
        assert_eq!(
            expected, actual,
            "No value was added to account: {} vs {}",
            expected, actual
        );
        Ok(())
    }
}

fn as_identifier(address: &Address) -> jormungandr_lib::crypto::account::Identifier {
    address.1.public_key().unwrap().clone().into()
}

use std::collections::HashMap;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum StateVerifierError {
    #[error("cannot find wallet in snapshot {0}")]
    NoWalletInSnapshot(String),
    #[error("no snapshot was made prior assert execution")]
    NoSnapshot,
    #[error("rest error")]
    RestError(#[from] super::RestError),
    #[error("wrong value calculation")]
    ValueError(#[from] chain_impl_mockchain::value::ValueError),
}

pub struct StateSnapshot {
    wallets: HashMap<String, AccountState>,
}

impl StateSnapshot {
    pub fn new(wallets: HashMap<String, AccountState>) -> Self {
        Self { wallets }
    }

    pub fn value_for(&self, address: &Address) -> Result<Value, StateVerifierError> {
        let address = address.to_string();
        let state = self
            .wallets
            .get(&address)
            .ok_or_else(|| StateVerifierError::NoWalletInSnapshot(address.clone()))?;
        Ok(*state.value())
    }
}
