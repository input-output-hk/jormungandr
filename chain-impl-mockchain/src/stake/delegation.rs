use crate::certificate::{Certificate, CertificateContent};
use imhamt::{Hamt, UpdateError};
use std::collections::hash_map::DefaultHasher;

use super::role::{StakeKeyId, StakeKeyInfo, StakePoolId, StakePoolInfo};

/// A structure that keeps track of stake keys and stake pools.
#[derive(Clone)]
pub struct DelegationState {
    pub(super) stake_keys: Hamt<DefaultHasher, StakeKeyId, StakeKeyInfo>,
    pub(super) stake_pools: Hamt<DefaultHasher, StakePoolId, StakePoolInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DelegationError {
    StakeKeyAlreadyRegistered,
    StakeKeyRegistrationSigIsInvalid,
    StakeKeyDeregistrationSigIsInvalid,
    StakeKeyDeregistrationDoesNotExist,
    StakeDelegationSigIsInvalid,
    StakeDelegationStakeKeyIsInvalid(StakeKeyId),
    StakeDelegationPoolKeyIsInvalid(StakePoolId),
    StakePoolRegistrationPoolSigIsInvalid,
    StakePoolAlreadyExists(StakePoolId),
    StakePoolRetirementSigIsInvalid,
    StakePoolDoesNotExist(StakePoolId),
}

impl std::fmt::Display for DelegationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DelegationError::StakeKeyAlreadyRegistered => write!(
                f,
                "Stake key already registered"
            ),
            DelegationError::StakeKeyRegistrationSigIsInvalid => write!(
                f,
                "Block has a stake key registration certificate with an invalid signature"
            ),
            DelegationError::StakeKeyDeregistrationSigIsInvalid => write!(
                f,
                "Block has a stake key deregistration certificate with an invalid signature"
            ),
            DelegationError::StakeKeyDeregistrationDoesNotExist => write!(
                f,
                "The Stake Key cannot be deregistered as it does not exist"
            ),
            DelegationError::StakeDelegationSigIsInvalid => write!(
                f,
                "Block has a stake delegation certificate with an invalid signature"
            ),
            DelegationError::StakeDelegationStakeKeyIsInvalid(stake_key_id) => write!(
                f,
                "Block has a stake delegation certificate that delegates from a stake key '{:?} that does not exist",
                stake_key_id
            ),
            DelegationError::StakeDelegationPoolKeyIsInvalid(pool_id) => write!(
                f,
                "Block has a stake delegation certificate that delegates to a pool '{:?} that does not exist",
                pool_id
            ),
            DelegationError::StakePoolRegistrationPoolSigIsInvalid => write!(
                f,
                "Block has a pool registration certificate with an invalid pool signature"
            ),
            DelegationError::StakePoolAlreadyExists(pool_id) => write!(
                f,
                "Block attempts to register pool '{:?}' which already exists",
                pool_id
            ),
            DelegationError::StakePoolRetirementSigIsInvalid => write!(
                f,
                "Block has a pool retirement certificate with an invalid pool signature"
            ),
            DelegationError::StakePoolDoesNotExist(pool_id) => write!(
                f,
                "Block references a pool '{:?}' which does not exist",
                pool_id
            ),
        }
    }
}

impl std::error::Error for DelegationError {}

impl DelegationState {
    pub fn new() -> Self {
        DelegationState {
            stake_keys: Hamt::new(),
            stake_pools: Hamt::new(),
        }
    }

    pub fn nr_stake_keys(&self) -> usize {
        self.stake_keys.size()
    }

    pub fn stake_key_exists(&self, stake_key_id: &StakeKeyId) -> bool {
        self.stake_keys
            .lookup(stake_key_id)
            .map_or_else(|| false, |_| true)
    }

    pub fn register_stake_key(&self, stake_key_id: StakeKeyId) -> Result<Self, DelegationError> {
        Ok(DelegationState {
            stake_keys: self
                .stake_keys
                .insert(stake_key_id, StakeKeyInfo { pool: None })
                .map_err(|_| DelegationError::StakeKeyAlreadyRegistered)?,
            stake_pools: self.stake_pools.clone(),
        })
    }

    pub fn deregister_stake_key(&self, stake_key_id: &StakeKeyId) -> Result<Self, DelegationError> {
        Ok(DelegationState {
            stake_keys: self
                .stake_keys
                .remove(&stake_key_id)
                .map_err(|_| DelegationError::StakeKeyDeregistrationDoesNotExist)?,
            stake_pools: self.stake_pools.clone(),
        })
    }

    //pub fn get_stake_pools(&self) -> &HashMap<GenesisPraosId, StakePoolInfo> {
    //    &self.stake_pools
    //}

    pub fn stake_pool_exists(&self, pool_id: &StakePoolId) -> bool {
        self.stake_pools
            .lookup(pool_id)
            .map_or_else(|| false, |_| true)
    }

    pub fn register_stake_pool(&self, owner: StakePoolInfo) -> Result<Self, DelegationError> {
        let id = owner.to_id();
        let new_pools = self
            .stake_pools
            .insert(id.clone(), owner)
            .map_err(|_| DelegationError::StakePoolAlreadyExists(id))?;
        Ok(DelegationState {
            stake_pools: new_pools,
            stake_keys: self.stake_keys.clone(),
        })
    }

    pub fn deregister_stake_pool(&self, pool_id: &StakePoolId) -> Result<Self, DelegationError> {
        Ok(DelegationState {
            stake_pools: self
                .stake_pools
                .remove(pool_id)
                .map_err(|_| DelegationError::StakePoolDoesNotExist(pool_id.clone()))?,
            stake_keys: self.stake_keys.clone(),
        })
    }

    pub fn delegate_stake(
        &self,
        stake_key_id: StakeKeyId,
        pool_id: StakePoolId,
    ) -> Result<Self, DelegationError> {
        let new_keys = self
            .stake_keys
            .update(&stake_key_id, |ki| {
                let mut kinfo = ki.clone();
                kinfo.pool = Some(pool_id);
                Ok(Some(kinfo))
            })
            // error mapping is wrong...
            .map_err(|_: UpdateError<()>| {
                DelegationError::StakeDelegationStakeKeyIsInvalid(stake_key_id.clone())
            })?;
        Ok(DelegationState {
            stake_keys: new_keys,
            stake_pools: self.stake_pools.clone(),
        })
    }

    pub(crate) fn apply(&self, certificate: &Certificate) -> Result<Self, DelegationError> {
        let mut new_state = self.clone();

        match certificate.content {
            CertificateContent::StakeDelegation(ref reg) => {
                if !self.stake_pool_exists(&reg.pool_id) {
                    return Err(DelegationError::StakeDelegationPoolKeyIsInvalid(
                        reg.pool_id.clone(),
                    ));
                }

                new_state =
                    new_state.delegate_stake(reg.stake_key_id.clone(), reg.pool_id.clone())?
            }
            CertificateContent::StakeKeyRegistration(ref reg) => {
                new_state = new_state.register_stake_key(reg.stake_key_id.clone())?
            }
            CertificateContent::StakeKeyDeregistration(ref reg) => {
                new_state = new_state.deregister_stake_key(&reg.stake_key_id)?
            }
            CertificateContent::StakePoolRegistration(ref reg) => {
                new_state = new_state.register_stake_pool(reg.clone())?
            }
            CertificateContent::StakePoolRetirement(ref reg) => {
                new_state = new_state.deregister_stake_pool(&reg.pool_id)?
            }
        }

        Ok(new_state)
    }
}
