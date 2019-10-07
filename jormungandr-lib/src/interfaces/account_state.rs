use crate::{crypto::hash::Hash, interfaces::Value};
use chain_impl_mockchain::accounting::account;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DelegationType {
    pools: Vec<(Hash, u8)>,
}

impl DelegationType {
    pub fn pools(&self) -> Vec<(Hash, u8)> {
        self.pools.clone()
    }
}

impl From<account::DelegationType> for DelegationType {
    fn from(dt: account::DelegationType) -> Self {
        match dt {
            account::DelegationType::NonDelegated => DelegationType { pools: Vec::new() },
            account::DelegationType::Full(h) => DelegationType {
                pools: vec![(h.into(), 1)],
            },
            account::DelegationType::Ratio(v) => DelegationType {
                pools: v
                    .pools
                    .iter()
                    .map(|(h, pp)| (h.clone().into(), *pp))
                    .collect(),
            },
        }
    }
}

impl From<DelegationType> for account::DelegationType {
    fn from(dt: DelegationType) -> Self {
        if dt.pools.len() == 0 {
            account::DelegationType::NonDelegated
        } else if dt.pools.len() == 1 {
            account::DelegationType::Full(dt.pools[0].0.into_digest_of())
        } else {
            let v: u32 = dt.pools.iter().map(|(_, i)| (*i as u32)).sum();
            match v.try_into() {
                Err(_) => panic!("delegation type pool overflow"),
                Ok(parts) => {
                    let ratio = account::DelegationRatio {
                        parts,
                        pools: dt
                            .pools
                            .iter()
                            .map(|(h, pp)| (h.into_digest_of(), *pp))
                            .collect(),
                    };
                    account::DelegationType::Ratio(ratio)
                }
            }
        }
    }
}

/// represent the current state of an account in the ledger
///
/// This type is different from the [`UTxOInfo`] which represents another
/// kind of mean to manipulate assets in the blockchain.
///
/// [`UTxOInfo`]: ./struct.UTxOInfo.html
///
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccountState {
    delegation: DelegationType,
    value: Value,
    counter: u32,
}

impl AccountState {
    /// retrieve the identifier to the stake pool this account is delegating its
    /// stake to.
    ///
    /// `None` means this account is not delegating its stake.
    #[inline]
    pub fn delegation(&self) -> &DelegationType {
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
            delegation: account.delegation().clone().into(),
            value: account.value().into(),
            counter: account.get_counter(),
        }
    }
}

impl<'a, E> From<&'a account::AccountState<E>> for AccountState {
    fn from(account: &'a account::AccountState<E>) -> Self {
        AccountState {
            delegation: account.delegation().clone().into(),
            value: account.value().into(),
            counter: account.get_counter(),
        }
    }
}
