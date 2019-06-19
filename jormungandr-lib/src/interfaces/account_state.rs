use crate::{crypto::hash::Hash, interfaces::Value};
use chain_impl_mockchain::accounting::account;
use serde::{Deserialize, Serialize};

/// represent the current state of an account in the ledger
///
/// This type is different from the [`UTxOInfo`] which represents another
/// kind of mean to manipulate assets in the blockchain.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccountState {
    delegation: Option<Hash>,
    value: Value,
    counter: u32,
}

impl AccountState {
    /// retrieve the identifier to the stake pool this account is delegating its
    /// stake to.
    ///
    /// `None` means this account is not delegating its stake.
    #[inline]
    pub fn delegation(&self) -> &Option<Hash> {
        &self.delegation
    }

    /// the current fund associated to this account
    #[inline]
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// the transaction counter. This is used as part of the parameter when adding
    /// a new account input to a transaction.
    ///
    #[inline]
    pub fn counter(&self) -> u32 {
        self.counter
    }
}

/* ---------------- Conversion --------------------------------------------- */

impl<E> From<account::AccountState<E>> for AccountState {
    fn from(account: account::AccountState<E>) -> Self {
        AccountState {
            delegation: account.delegation().clone().map(|h| {
                let h: [u8; 32] = h.into();
                h.into()
            }),
            value: account.value().into(),
            counter: account.get_counter(),
        }
    }
}

impl<'a, E> From<&'a account::AccountState<E>> for AccountState {
    fn from(account: &'a account::AccountState<E>) -> Self {
        AccountState {
            delegation: account.delegation().clone().map(|h| {
                let h: [u8; 32] = h.into();
                h.into()
            }),
            value: account.value().into(),
            counter: account.get_counter(),
        }
    }
}
