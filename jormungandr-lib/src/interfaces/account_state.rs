use super::mint_token::TokenIdentifier;
use crate::{crypto::hash::Hash, interfaces::Value};
use chain_impl_mockchain::{accounting::account, block::Epoch};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, convert::TryInto};

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
                    .pools()
                    .iter()
                    .map(|(h, pp)| (h.clone().into(), *pp))
                    .collect(),
            },
        }
    }
}

impl From<DelegationType> for account::DelegationType {
    fn from(dt: DelegationType) -> Self {
        if dt.pools.is_empty() {
            account::DelegationType::NonDelegated
        } else if dt.pools.len() == 1 {
            account::DelegationType::Full(dt.pools[0].0.into_digest_of())
        } else {
            let v: u32 = dt.pools.iter().map(|(_, i)| (*i as u32)).sum();
            match v.try_into() {
                Err(error) => panic!("delegation type pool overflow: {}", error),
                Ok(parts) => {
                    let ratio = account::DelegationRatio::new(
                        parts,
                        dt.pools()
                            .iter()
                            .map(|(h, pp)| (h.into_digest_of(), *pp))
                            .collect(),
                    )
                    .expect("Assume this is always correct for a delegation type");
                    account::DelegationType::Ratio(ratio)
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LastRewards {
    epoch: Epoch,
    reward: Value,
}

impl LastRewards {
    #[inline]
    pub fn epoch(&self) -> &Epoch {
        &self.epoch
    }

    #[inline]
    pub fn reward(&self) -> &Value {
        &self.reward
    }
}

/// represent the current state of an account in the ledger
///
/// This type is different from the [`UTxOInfo`] which represents another
/// kind of mean to manipulate assets in the blockchain.
///
/// [`UTxOInfo`]: ./struct.UTxOInfo.html
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountState {
    delegation: DelegationType,
    value: Value,
    counters: Vec<u32>,
    tokens: BTreeMap<TokenIdentifier, Value>,
    last_rewards: LastRewards,
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

    /// The transaction counters for spending lanes.
    /// A counter in one of the existing lanes is used as part of the parameter
    /// when adding a new account input to a transaction.
    ///
    #[inline]
    pub fn counters(&self) -> Vec<u32> {
        self.counters.clone()
    }

    /// the last rewards transfered to account
    #[inline]
    pub fn last_rewards(&self) -> &LastRewards {
        &self.last_rewards
    }

    /// the current tokens associated to this account
    #[inline]
    pub fn tokens(&self) -> &BTreeMap<TokenIdentifier, Value> {
        &self.tokens
    }
}

/* ---------------- Conversion --------------------------------------------- */

impl From<account::LastRewards> for LastRewards {
    fn from(lr: account::LastRewards) -> Self {
        Self {
            epoch: lr.epoch,
            reward: lr.reward.into(),
        }
    }
}

impl From<LastRewards> for account::LastRewards {
    fn from(lr: LastRewards) -> Self {
        Self {
            epoch: lr.epoch,
            reward: lr.reward.into(),
        }
    }
}

impl<E> From<account::AccountState<E>> for AccountState {
    fn from(account: account::AccountState<E>) -> Self {
        AccountState {
            delegation: account.delegation().clone().into(),
            value: account.value().into(),
            counters: account
                .spending
                .get_valid_counters()
                .into_iter()
                .map(Into::into)
                .collect(),
            tokens: account
                .tokens
                .iter()
                .map(|(identifier, value)| {
                    (
                        TokenIdentifier::from(identifier.clone()),
                        Value::from(*value),
                    )
                })
                .collect(),
            last_rewards: account.last_rewards.into(),
        }
    }
}

impl<'a, E> From<&'a account::AccountState<E>> for AccountState {
    fn from(account: &'a account::AccountState<E>) -> Self {
        AccountState {
            delegation: account.delegation().clone().into(),
            value: account.value().into(),
            counters: account
                .spending
                .get_valid_counters()
                .into_iter()
                .map(Into::into)
                .collect(),
            tokens: account
                .tokens
                .iter()
                .map(|(identifier, value)| {
                    (
                        TokenIdentifier::from(identifier.clone()),
                        Value::from(*value),
                    )
                })
                .collect(),
            last_rewards: account.last_rewards.clone().into(),
        }
    }
}
