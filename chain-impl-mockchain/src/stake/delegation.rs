use crate::certificate::{PoolId, PoolRegistration};
use crate::transaction::AccountIdentifier;
use imhamt::Hamt;
use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Debug};

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

impl Debug for DelegationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}",
            self.stake_pools
                .iter()
                .map(|(id, stake)| (id.clone(), stake.clone()))
                .collect::<Vec<(PoolId, PoolRegistration)>>()
        )
    }
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

    pub fn stake_pool_ids<'a>(&'a self) -> impl Iterator<Item = PoolId> + 'a {
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

#[cfg(test)]
mod tests {

    use super::DelegationState;
    use crate::certificate::{PoolId, PoolRegistration};
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;
    use std::iter;

    impl Arbitrary for DelegationState {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let size = usize::arbitrary(gen);
            let arbitrary_stake_pools = iter::from_fn(|| Some(PoolRegistration::arbitrary(gen)))
                .take(size)
                .collect::<Vec<PoolRegistration>>();
            let mut delegation_state = DelegationState::new();
            for stake_pool in arbitrary_stake_pools {
                delegation_state = delegation_state.register_stake_pool(stake_pool).unwrap();
            }
            delegation_state
        }
    }

    #[quickcheck]
    pub fn delegation_state_tests(
        delegation_state: DelegationState,
        stake_pool: PoolRegistration,
    ) -> TestResult {
        // register stake pool first time should be ok
        let delegation_state = match delegation_state.register_stake_pool(stake_pool.clone()) {
            Ok(delegation_state) => delegation_state,
            Err(err) => {
                return TestResult::error(format!("Cannot register stake pool, due to {:?}", err))
            }
        };

        // register stake pool again should throw error
        if delegation_state
            .register_stake_pool(stake_pool.clone())
            .is_ok()
        {
            return TestResult::error(
                "Register the same stake pool twice should return error while it didn't",
            );
        }

        let stake_pool_id = stake_pool.to_id();

        // stake pool should be in collection
        if !delegation_state
            .stake_pool_ids()
            .any(|x| x == stake_pool_id)
        {
            return TestResult::error(format!(
                "stake pool with id: {:?} should exist in iterator",
                stake_pool_id
            ));
        };

        // stake pool should exist in collection
        if !delegation_state.stake_pool_exists(&stake_pool_id) {
            TestResult::error(format!(
                "stake pool with id {:?} should exist in collection",
                stake_pool_id
            ));
        }

        // deregister stake pool should be ok
        let delegation_state = match delegation_state.deregister_stake_pool(&stake_pool_id) {
            Ok(delegation_state) => delegation_state,
            Err(err) => {
                return TestResult::error(format!("Cannot deregister stake pool due to: {:?}", err))
            }
        };

        // deregister stake pool again should throw error
        if delegation_state
            .deregister_stake_pool(&stake_pool_id)
            .is_ok()
        {
            return TestResult::error(
                "Deregister the same stake pool twice should return error while it didn't",
            );
        }

        // stake pool should not exist in collection
        if delegation_state.stake_pool_exists(&stake_pool_id) {
            return TestResult::error(format!(
                "stake pool with id should be removed from collection {:?}",
                stake_pool_id
            ));
        }

        // stake pool should not be in collection
        if delegation_state
            .stake_pool_ids()
            .any(|x| x == stake_pool_id)
        {
            return TestResult::error(format!(
                "stake pool with id should be removed from iterator {:?}",
                stake_pool_id
            ));
        }

        TestResult::passed()
    }
}
