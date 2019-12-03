use crate::certificate::{PoolId, PoolRegistration};
use crate::header::Epoch;
use crate::value::Value;
use imhamt::Hamt;
use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Debug};
use std::sync::Arc;

/// A structure that keeps track of stake keys and stake pools.
#[derive(Clone, PartialEq, Eq)]
pub struct PoolsState {
    pub(crate) stake_pools: Hamt<DefaultHasher, PoolId, PoolState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    AlreadyExists(PoolId),
    NotFound(PoolId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolLastRewards {
    pub epoch: Epoch,
    pub value_taxed: Value,
    pub value_for_stakers: Value,
}

impl PoolLastRewards {
    pub fn default() -> Self {
        PoolLastRewards {
            epoch: 0,
            value_taxed: Value::zero(),
            value_for_stakers: Value::zero(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolState {
    pub last_rewards: PoolLastRewards,
    pub registration: Arc<PoolRegistration>,
}

impl PoolState {
    pub fn new(reg: PoolRegistration) -> Self {
        PoolState {
            last_rewards: PoolLastRewards::default(),
            registration: Arc::new(reg),
        }
    }
}

impl Debug for PoolsState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}",
            self.stake_pools
                .iter()
                .map(|(id, stake)| (id.clone(), stake.clone()))
                .collect::<Vec<(PoolId, PoolState)>>()
        )
    }
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PoolError::AlreadyExists(pool_id) => write!(
                f,
                "Block attempts to register pool '{:?}' which already exists",
                pool_id
            ),
            PoolError::NotFound(pool_id) => write!(
                f,
                "Block references a pool '{:?}' which does not exist",
                pool_id
            ),
        }
    }
}

impl std::error::Error for PoolError {}

impl PoolsState {
    pub fn new() -> Self {
        PoolsState {
            stake_pools: Hamt::new(),
        }
    }

    pub fn lookup(&self, id: &PoolId) -> Option<&PoolState> {
        self.stake_pools.lookup(id)
    }

    pub fn lookup_reg(&self, id: &PoolId) -> Option<&PoolRegistration> {
        self.stake_pools.lookup(id).map(|x| x.registration.as_ref())
    }

    pub fn stake_pool_ids<'a>(&'a self) -> impl Iterator<Item = PoolId> + 'a {
        self.stake_pools.iter().map(|(id, _)| id.clone())
    }

    pub fn stake_pool_exists(&self, pool_id: &PoolId) -> bool {
        self.stake_pools
            .lookup(pool_id)
            .map_or_else(|| false, |_| true)
    }

    pub fn stake_pool_get(&self, pool_id: &PoolId) -> Result<&PoolRegistration, PoolError> {
        self.stake_pools
            .lookup(pool_id)
            .ok_or(PoolError::NotFound(pool_id.clone()))
            .map(|s| s.registration.as_ref())
    }

    pub fn stake_pool_set_rewards(
        &mut self,
        pool_id: &PoolId,
        epoch: Epoch,
        value_taxed: Value,
        value_for_stakers: Value,
    ) -> Result<(), PoolError> {
        let rw = PoolLastRewards {
            epoch,
            value_taxed,
            value_for_stakers,
        };
        self.stake_pools = self
            .stake_pools
            .replace_with(pool_id, |st| {
                let mut st = st.clone();
                st.last_rewards = rw;
                st
            })
            .map_err(|_| PoolError::NotFound(pool_id.clone()))?;
        Ok(())
    }

    pub fn register_stake_pool(&self, owner: PoolRegistration) -> Result<Self, PoolError> {
        let id = owner.to_id();
        let new_pools = self
            .stake_pools
            .insert(id.clone(), PoolState::new(owner))
            .map_err(|_| PoolError::AlreadyExists(id))?;
        Ok(PoolsState {
            stake_pools: new_pools,
        })
    }

    pub fn deregister_stake_pool(&self, pool_id: &PoolId) -> Result<Self, PoolError> {
        Ok(PoolsState {
            stake_pools: self
                .stake_pools
                .remove(pool_id)
                .map_err(|_| PoolError::NotFound(pool_id.clone()))?,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::PoolsState;
    use crate::certificate::PoolRegistration;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;
    use std::iter;

    impl Arbitrary for PoolsState {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let size = usize::arbitrary(gen);
            let arbitrary_stake_pools = iter::from_fn(|| Some(PoolRegistration::arbitrary(gen)))
                .take(size)
                .collect::<Vec<PoolRegistration>>();
            let mut delegation_state = PoolsState::new();
            for stake_pool in arbitrary_stake_pools {
                delegation_state = delegation_state.register_stake_pool(stake_pool).unwrap();
            }
            delegation_state
        }
    }

    #[quickcheck]
    pub fn delegation_state_tests(
        delegation_state: PoolsState,
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
