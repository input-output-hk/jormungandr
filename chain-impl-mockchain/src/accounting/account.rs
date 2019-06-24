//! Generic account like accounting
//!
//! This is effectively an immutable clonable-HAMT of bank style account,
//! which contains a non negative value representing your balance with the
//! identifier of this account as key.

use crate::stake::StakePoolId;
use crate::value::*;
use imhamt::{Hamt, HamtIter, InsertError, UpdateError};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;

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

#[derive(Clone)]
pub struct AccountState<Extra> {
    pub counter: SpendingCounter,
    pub delegation: Option<StakePoolId>,
    pub value: Value,
    pub extra: Extra,
}

impl<Extra> AccountState<Extra> {
    /// Create a new account state with a specific start value
    pub fn new(v: Value, e: Extra) -> Self {
        Self {
            counter: SpendingCounter(0),
            delegation: None,
            value: v,
            extra: e,
        }
    }

    /// Get referencet to delegation setting
    pub fn delegation(&self) -> &Option<StakePoolId> {
        &self.delegation
    }

    pub fn value(&self) -> Value {
        self.value
    }

    // deprecated use value()
    pub fn get_value(&self) -> Value {
        self.value
    }

    pub fn get_counter(&self) -> u32 {
        self.counter.into()
    }
}

impl<Extra: Clone> AccountState<Extra> {
    /// Add a value to an account state
    ///
    /// Only error if value is overflowing
    pub fn add(&self, v: Value) -> Result<Self, LedgerError> {
        let new_value = (self.value + v)?;
        let mut st = self.clone();
        st.value = new_value;
        Ok(st)
    }

    /// Subtract a value from an account state, and return the new state.
    ///
    /// Note that this *also* increment the counter, as this function would be usually call
    /// for spending.
    ///
    /// If the counter is also reaching the extremely rare of max, we only authorise
    /// a total withdrawal of fund otherwise the fund will be stuck forever in limbo.
    pub fn sub(&self, v: Value) -> Result<Option<Self>, LedgerError> {
        let new_value = (self.value - v)?;
        match self.counter.increment() {
            None => {
                if new_value == Value::zero() {
                    Ok(None)
                } else {
                    Err(LedgerError::NeedTotalWithdrawal)
                }
            }
            Some(new_counter) => Ok(Some(Self {
                counter: new_counter,
                delegation: self.delegation.clone(),
                value: new_value,
                extra: self.extra.clone(),
            })),
        }
    }

    /// Set delegation
    pub fn set_delegation(&self, delegation: Option<StakePoolId>) -> Self {
        let mut st = self.clone();
        st.delegation = delegation;
        st
    }
}

/// Spending counter associated to an account.
///
/// every time the owner is spending from an account,
/// the counter is incremented. A matching counter
/// needs to be used in the spending phase to make
/// sure we have non-replayability of a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendingCounter(u32);

impl SpendingCounter {
    pub fn zero() -> Self {
        SpendingCounter(0)
    }

    fn increment(&self) -> Option<Self> {
        self.0.checked_add(1).map(SpendingCounter)
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

impl From<u32> for SpendingCounter {
    fn from(v: u32) -> Self {
        SpendingCounter(v)
    }
}

impl From<SpendingCounter> for u32 {
    fn from(v: SpendingCounter) -> u32 {
        v.0
    }
}

pub struct Iter<'a, ID, Extra>(HamtIter<'a, ID, AccountState<Extra>>);

impl<'a, ID, Extra> Iterator for Iter<'a, ID, Extra> {
    type Item = (&'a ID, &'a AccountState<Extra>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// The public ledger of all accounts associated with their current state
#[derive(Clone)]
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
