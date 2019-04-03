use crate::key;
use crate::value::*;
use chain_crypto::{Ed25519Extended, PublicKey};
use imhamt::{Hamt, InsertError, UpdateError};
use std::collections::hash_map::DefaultHasher;

pub type AccountAlg = Ed25519Extended;

/// Possible errors during an account operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    NonExistent,
    AlreadyExists,
    MismatchCounter,
    NeedTotalWithdrawal,
    NonZero,
    ValueError(ValueError),
}

impl From<ValueError> for LedgerError {
    fn from(e: ValueError) -> Self {
        LedgerError::ValueError(e)
    }
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

/// Account Identifier (also used as Public Key)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier(PublicKey<AccountAlg>);

impl From<PublicKey<AccountAlg>> for Identifier {
    fn from(pk: PublicKey<AccountAlg>) -> Self {
        Identifier(pk)
    }
}

impl From<Identifier> for PublicKey<AccountAlg> {
    fn from(i: Identifier) -> Self {
        i.0
    }
}

/// Account Secret Key
pub type Secret = key::AccountSecretKey;

#[derive(Clone)]
pub struct State {
    counter: SpendingCounter,
    value: Value,
}

impl State {
    pub fn new(v: Value) -> State {
        State {
            counter: SpendingCounter(0),
            value: v,
        }
    }

    /// Add a value to an account state
    ///
    /// Only error if value is overflowing
    pub fn add(&self, v: Value) -> Result<State, LedgerError> {
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
    pub fn sub(&self, v: Value) -> Result<Option<State>, LedgerError> {
        let new_value = (self.value - v)?;
        match self.counter.increment() {
            None => {
                if new_value == Value::zero() {
                    Ok(None)
                } else {
                    Err(LedgerError::NeedTotalWithdrawal)
                }
            }
            Some(new_counter) => Ok(Some(State {
                counter: new_counter,
                value: new_value,
            })),
        }
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
    fn increment(&self) -> Option<Self> {
        self.0.checked_add(1).map(SpendingCounter)
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

/// Account Spending witness, which contains a
/// cryptographic signature and a counter.
/// The counter need to be matched with the current state of this account in the ledger,
/// otherwise the transaction will not be valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendingWitness {
    counter: SpendingCounter,
    signature: u32, // FIXME
}

/// The public ledger of all accounts associated with their current state
#[derive(Clone)]
pub struct Ledger(Hamt<DefaultHasher, Identifier, State>);

impl Ledger {
    /// Create a new empty account ledger
    pub fn new() -> Self {
        Ledger(Hamt::new())
    }

    /// Add a new account into this ledger.
    ///
    /// If the identifier is already present, error out.
    pub fn add_account(
        &self,
        account: &Identifier,
        initial_value: Value,
    ) -> Result<Self, LedgerError> {
        self.0
            .insert(account.clone(), State::new(initial_value))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// Remove an account from this ledger
    ///
    /// If the account still have value > 0, then error
    pub fn remove_account(&self, account: &Identifier) -> Result<Self, LedgerError> {
        self.0
            .update(account, |st| {
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
    pub fn add_value(&self, account: &Identifier, value: Value) -> Result<Self, LedgerError> {
        self.0
            .update(account, |st| st.add(value).map(Some))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// Subtract value to an existing account.
    ///
    /// If the account doesn't exist, or that the value would become negative, errors out.
    pub fn remove_value(
        &self,
        account: &Identifier,
        value: Value,
    ) -> Result<(Self, SpendingCounter), LedgerError> {
        // ideally we don't need 2 calls to do this
        let counter = self
            .0
            .lookup(account)
            .map_or(Err(LedgerError::NonExistent), |st| Ok(st.counter))?;
        self.0
            .update(account, |st| st.sub(value))
            .map(|ledger| (Ledger(ledger), counter))
            .map_err(|e| e.into())
    }
}
