use crate::{crypto::hash::Hash, interfaces::Value};
use chain_impl_mockchain::accounting::account;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccountState {
    delegation: Option<Hash>,
    value: Value,
    counter: u32,
}

impl AccountState {
    #[inline]
    pub fn delegation(&self) -> &Option<Hash> {
        &self.delegation
    }

    #[inline]
    pub fn value(&self) -> &Value {
        &self.value
    }

    #[inline]
    pub fn counter(&self) -> &u32 {
        &self.counter
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
