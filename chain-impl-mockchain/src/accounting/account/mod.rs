//! Generic account like accounting
//!
//! This is effectively an immutable clonable-HAMT of bank style account,
//! which contains a non negative value representing your balance with the
//! identifier of this account as key.

use crate::stake::StakePoolId;
use crate::value::*;
use imhamt::{Hamt, InsertError, UpdateError};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;

pub mod account_state;

pub use account_state::*;

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub LedgerError
        NonExistent = "Account does not exist",
        AlreadyExists = "Account already exists",
        NeedTotalWithdrawal = "Operation counter reached its maximum and next operation must be full withdrawal",
        NonZero = "Removed account is not empty",
        ValueError{ source: ValueError } = "Value calculation failed",
}

impl From<UpdateError<LedgerError>> for LedgerError {
    fn from(e: UpdateError<LedgerError>) -> Self {
        match e {
            UpdateError::KeyNotFound => LedgerError::NonExistent,
            UpdateError::ValueCallbackError(v) => v,
        }
    }
}

impl From<InsertError> for LedgerError {
    fn from(e: InsertError) -> Self {
        match e {
            InsertError::EntryExists => LedgerError::AlreadyExists,
        }
    }
}

/// The public ledger of all accounts associated with their current state
#[derive(Clone, PartialEq, Eq)]
pub struct Ledger<ID: Hash + Eq, Extra>(Hamt<DefaultHasher, ID, AccountState<Extra>>);

impl<ID: Clone + Eq + Hash, Extra: Clone> Ledger<ID, Extra> {
    /// Create a new empty account ledger
    pub fn new() -> Self {
        Ledger(Hamt::new())
    }

    /// Add a new account into this ledger.
    ///
    /// If the identifier is already present, error out.
    pub fn add_account(
        &self,
        identifier: &ID,
        initial_value: Value,
        extra: Extra,
    ) -> Result<Self, LedgerError> {
        self.0
            .insert(identifier.clone(), AccountState::new(initial_value, extra))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// Set the delegation of an account in this ledger
    pub fn set_delegation(
        &self,
        identifier: &ID,
        delegation: Option<StakePoolId>,
    ) -> Result<Self, LedgerError> {
        self.0
            .update(identifier, |st| Ok(Some(st.set_delegation(delegation))))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// check if an account already exist
    #[inline]
    pub fn exists(&self, identifier: &ID) -> bool {
        self.0.contains_key(identifier)
    }

    /// Get account state
    ///
    /// If the identifier does not match any account, error out
    pub fn get_state(&self, account: &ID) -> Result<&AccountState<Extra>, LedgerError> {
        self.0.lookup(account).ok_or(LedgerError::NonExistent)
    }

    /// Remove an account from this ledger
    ///
    /// If the account still have value > 0, then error
    pub fn remove_account(&self, identifier: &ID) -> Result<Self, LedgerError> {
        self.0
            .update(identifier, |st| {
                if st.value == Value::zero() {
                    Ok(None)
                } else {
                    Err(LedgerError::NonZero)
                }
            })
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// Add value to an existing account.
    ///
    /// If the account doesn't exist, error out.
    pub fn add_value(&self, identifier: &ID, value: Value) -> Result<Self, LedgerError> {
        self.0
            .update(identifier, |st| st.add(value).map(Some))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// Subtract value to an existing account.
    ///
    /// If the account doesn't exist, or that the value would become negative, errors out.
    pub fn remove_value(
        &self,
        identifier: &ID,
        value: Value,
    ) -> Result<(Self, SpendingCounter), LedgerError> {
        // ideally we don't need 2 calls to do this
        let counter = self
            .0
            .lookup(identifier)
            .map_or(Err(LedgerError::NonExistent), |st| Ok(st.counter))?;
        self.0
            .update(identifier, |st| st.sub(value))
            .map(|ledger| (Ledger(ledger), counter))
            .map_err(|e| e.into())
    }

    pub fn get_total_value(&self) -> Result<Value, ValueError> {
        let values = self
            .0
            .iter()
            .map(|(_, account_state)| account_state.get_value());
        Value::sum(values)
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, ID, Extra> {
        Iter(self.0.iter())
    }
}

impl<ID: Clone + Eq + Hash, Extra: Clone> std::iter::FromIterator<(ID, AccountState<Extra>)>
    for Ledger<ID, Extra>
{
    fn from_iter<I: IntoIterator<Item = (ID, AccountState<Extra>)>>(iter: I) -> Self {
        Ledger(Hamt::from_iter(iter))
    }
}

#[cfg(test)]
pub mod tests {

    use crate::account::Identifier;

    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    use super::AccountState;
    use crate::account::Ledger;
    use crate::value::Value;

    #[quickcheck]
    pub fn ledger_total_value_is_correct_after_remove_value(
        id: Identifier,
        account_state: AccountState<()>,
        value_to_remove: Value,
    ) -> TestResult {
        let mut ledger = Ledger::new();
        ledger = ledger
            .add_account(&id, account_state.get_value(), ())
            .unwrap();
        let result = ledger.remove_value(&id, value_to_remove);
        let expected_result = account_state.get_value() - value_to_remove;
        match (result, expected_result) {
            (Err(_), Err(_)) => verify_total_value(ledger, account_state.get_value()),
            (Ok(_), Err(_)) => TestResult::failed(),
            (Err(_), Ok(_)) => TestResult::failed(),
            (Ok((ledger, _)), Ok(value)) => verify_total_value(ledger, value),
        }
    }

    fn verify_total_value(ledger: Ledger, value: Value) -> TestResult {
        match ledger.get_total_value().unwrap() == value {
            true => TestResult::passed(),
            false => TestResult::error(format!(
                "Wrong total value got {:?}, while expecting {:?}",
                ledger.get_total_value(),
                value
            )),
        }
    }

    #[quickcheck]
    pub fn ledger_removes_account_only_if_zeroed(
        id: Identifier,
        account_state: AccountState<()>,
    ) -> TestResult {
        let mut ledger = Ledger::new();
        ledger = ledger
            .add_account(&id, account_state.get_value(), ())
            .unwrap();
        let result = ledger.remove_account(&id);
        let expected_result = account_state.get_value() == Value::zero();
        match (result, expected_result) {
            (Err(_), false) => verify_account_exists(&ledger, &id),
            (Ok(_), false) => TestResult::failed(),
            (Err(_), true) => TestResult::failed(),
            (Ok(ledger), true) => verify_account_does_not_exist(&ledger, &id),
        }
    }

    fn verify_account_exists(ledger: &Ledger, id: &Identifier) -> TestResult {
        match ledger.exists(&id) {
            true => TestResult::passed(),
            false => TestResult::error(format!("Account ({:?}) not exists , while it should", &id)),
        }
    }

    fn verify_account_does_not_exist(ledger: &Ledger, id: &Identifier) -> TestResult {
        match ledger.exists(&id) {
            true => TestResult::error(format!("Account ({:?}) exists , while it should not", &id)),
            false => TestResult::passed(),
        }
    }
}
