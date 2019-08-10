use imhamt::Hamt;
use std::collections::hash_map::DefaultHasher;

use crate::certificate::{PoolId, PoolRegistration};
use crate::transaction::AccountIdentifier;

/// All registered Stake Node
pub type PoolTable = Hamt<DefaultHasher, PoolId, PoolRegistration>;

/// A structure that keeps track of stake keys and stake pools.
#[derive(Clone, PartialEq, Eq)]
pub struct DelegationState {
    pub(crate) stake_pools: PoolTable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DelegationError {
    StakeDelegationSigIsInvalid,
    StakeDelegationPoolKeyIsInvalid(PoolId),
    StakeDelegationAccountIsInvalid(AccountIdentifier),
    StakePoolRegistrationPoolSigIsInvalid,
    StakePoolAlreadyExists(PoolId),
    StakePoolRetirementSigIsInvalid,
    StakePoolDoesNotExist(PoolId),
}

impl std::fmt::Display for DelegationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DelegationError::StakeDelegationSigIsInvalid => write!(
                f,
                "Block has a stake delegation certificate with an invalid signature"
            ),
            DelegationError::StakeDelegationPoolKeyIsInvalid(pool_id) => write!(
                f,
                "Block has a stake delegation certificate that delegates to a pool '{:?} that does not exist",
                pool_id
            ),
            DelegationError::StakeDelegationAccountIsInvalid(account_id) => write!(
                f,
                "Block has a stake delegation certificate that delegates from an account '{:?} that does not exist",
                account_id
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
            stake_pools: Hamt::new(),
        }
    }

    pub fn stake_pool_ids<'a>(&'a self) -> impl Iterator<Item = StakePoolId> + 'a {
        self.stake_pools.iter().map(|(id, _)| id.clone())
    }

    pub fn stake_pool_exists(&self, pool_id: &PoolId) -> bool {
        self.stake_pools
            .lookup(pool_id)
            .map_or_else(|| false, |_| true)
    }

    pub fn stake_pool_lookup(&self, pool_id: &PoolId) -> Option<&PoolRegistration> {
        self.stake_pools.lookup(pool_id)
    }

    pub fn stake_pool_get(&self, pool_id: &PoolId) -> Result<&PoolRegistration, DelegationError> {
        self.stake_pools
            .lookup(pool_id)
            .ok_or(DelegationError::StakePoolDoesNotExist(pool_id.clone()))
    }

    pub fn register_stake_pool(&self, owner: PoolRegistration) -> Result<Self, DelegationError> {
        let id = owner.to_id();
        let new_pools = self
            .stake_pools
            .insert(id.clone(), owner)
            .map_err(|_| DelegationError::StakePoolAlreadyExists(id))?;
        Ok(DelegationState {
            stake_pools: new_pools,
        })
    }

    pub fn deregister_stake_pool(&self, pool_id: &PoolId) -> Result<Self, DelegationError> {
        Ok(DelegationState {
            stake_pools: self
                .stake_pools
                .remove(pool_id)
                .map_err(|_| DelegationError::StakePoolDoesNotExist(pool_id.clone()))?,
        })
    }
}
