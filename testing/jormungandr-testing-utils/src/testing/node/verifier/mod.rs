mod fragment_log;

pub use fragment_log::{assert_accepted_rejected, assert_bad_request, FragmentLogVerifier};

use super::JormungandrRest;
use crate::wallet::Wallet;
use jormungandr_lib::interfaces::{AccountState, Value};

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

    pub fn record_wallets_state(mut self, wallets: Vec<&Wallet>) -> Self {
        self.snapshot_before = Some(StateSnapshot::new(
            wallets
                .iter()
                .map(|w| {
                    (
                        w.address().to_string(),
                        self.rest
                            .account_state(w)
                            .expect("cannot rerieve account state"),
                    )
                })
                .collect(),
        ));
        self
    }

    pub fn value_moved_between_wallets(
        &self,
        from: &Wallet,
        to: &Wallet,
        value: Value,
    ) -> Result<(), StateVerifierError> {
        self.wallet_lost_value(&from, value)?;
        self.wallet_gain_value(&to, value)?;
        Ok(())
    }

    pub fn wallet_lost_value(
        &self,
        wallet: &Wallet,
        value: Value,
    ) -> Result<(), StateVerifierError> {
        let snapshot = self
            .snapshot_before
            .as_ref()
            .ok_or(StateVerifierError::NoSnapshot)?;
        let expected = snapshot.value_for(wallet)?;
        let actual = self
            .rest
            .account_state(wallet)?
            .value()
            .checked_add(value)?;
        assert_eq!(
            expected, actual,
            "No value was deducted friom account: {} vs {}",
            expected, actual
        );
        Ok(())
    }

    pub fn no_changes(&self, wallets: Vec<&Wallet>) -> Result<(), StateVerifierError> {
        for wallet in wallets {
            self.wallet_has_the_same_value(wallet)?;
        }
        Ok(())
    }

    pub fn wallet_has_the_same_value(&self, wallet: &Wallet) -> Result<(), StateVerifierError> {
        let snapshot = self
            .snapshot_before
            .as_ref()
            .ok_or(StateVerifierError::NoSnapshot)?;
        let expected = snapshot.value_for(wallet)?;
        let actual = *self.rest.account_state(wallet)?.value();
        assert_eq!(
            expected, actual,
            "value changed for account {:?}: {} vs {}",
            wallet, expected, actual
        );
        Ok(())
    }

    pub fn wallet_gain_value(
        &self,
        wallet: &Wallet,
        value: Value,
    ) -> Result<(), StateVerifierError> {
        let snapshot = self
            .snapshot_before
            .as_ref()
            .ok_or(StateVerifierError::NoSnapshot)?;
        let expected = snapshot.value_for(wallet)?.checked_add(value)?;
        let actual = *self.rest.account_state(wallet)?.value();
        assert_eq!(
            expected, actual,
            "No value was added to account: {} vs {}",
            expected, actual
        );
        Ok(())
    }
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

    pub fn value_for(&self, wallet: &Wallet) -> Result<Value, StateVerifierError> {
        let address = wallet.address().to_string();
        let state = self
            .wallets
            .get(&address)
            .ok_or_else(|| StateVerifierError::NoWalletInSnapshot(address.clone()))?;
        Ok(*state.value())
    }
}
