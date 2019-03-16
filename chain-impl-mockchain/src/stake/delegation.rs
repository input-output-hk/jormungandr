use chain_crypto::{algorithms::vrf::vrf, FakeMMM, PublicKey};
use std::collections::{HashMap, HashSet};

use super::role::{StakeKeyId, StakeKeyInfo, StakePoolId, StakePoolInfo};

/// A structure that keeps track of stake keys and stake pools.
#[derive(Debug)]
pub struct DelegationState {
    pub(super) stake_keys: HashMap<StakeKeyId, StakeKeyInfo>,
    pub(super) stake_pools: HashMap<StakePoolId, StakePoolInfo>,
}

impl DelegationState {
    pub fn new(
        initial_stake_pools: Vec<StakePoolInfo>,
        initial_stake_keys: HashMap<StakeKeyId, Option<StakePoolId>>,
    ) -> Self {
        let mut stake_pools: HashMap<StakePoolId, StakePoolInfo> = initial_stake_pools
            .into_iter()
            .map(|pool_info| (pool_info.pool_id.clone(), pool_info))
            .collect();

        let mut stake_keys = HashMap::new();
        for (stake_key_id, pool_id) in initial_stake_keys {
            if let Some(pool_id) = &pool_id {
                if let Some(pool) = stake_pools.get_mut(&pool_id) {
                    pool.members.insert(stake_key_id.clone());
                } else {
                    panic!("Pool '{:?}' does not exist.", pool_id)
                }
            }
            stake_keys.insert(stake_key_id, StakeKeyInfo { pool: pool_id });
        }

        DelegationState {
            stake_keys,
            stake_pools,
        }
    }

    pub fn nr_stake_keys(&self) -> usize {
        self.stake_keys.len()
    }

    pub fn stake_key_exists(&self, stake_key_id: &StakeKeyId) -> bool {
        self.stake_keys.contains_key(stake_key_id)
    }

    pub fn register_stake_key(&mut self, stake_key_id: StakeKeyId) {
        let inserted = !self
            .stake_keys
            .insert(stake_key_id, StakeKeyInfo { pool: None })
            .is_some();
        assert!(inserted);
    }

    pub fn deregister_stake_key(&mut self, stake_key_id: &StakeKeyId) {
        let stake_key_info = self.stake_keys.remove(&stake_key_id).unwrap();

        // Remove this stake key from its pool, if any.
        if let Some(pool_id) = stake_key_info.pool {
            self.stake_pools
                .get_mut(&pool_id)
                .unwrap()
                .members
                .remove(&stake_key_id);
        }
    }

    pub fn get_stake_pools(&self) -> &HashMap<StakePoolId, StakePoolInfo> {
        &self.stake_pools
    }

    pub fn stake_pool_exists(&self, pool_id: &StakePoolId) -> bool {
        self.stake_pools.contains_key(pool_id)
    }

    pub fn register_stake_pool(
        &mut self,
        pool_id: StakePoolId,
        kes_public_key: PublicKey<FakeMMM>,
        vrf_public_key: vrf::PublicKey,
    ) {
        assert!(!self.stake_pools.contains_key(&pool_id));
        self.stake_pools.insert(
            pool_id.clone(),
            StakePoolInfo {
                pool_id: pool_id,
                //owners: new_stake_pool.owners
                members: HashSet::new(),
                kes_public_key: kes_public_key,
                vrf_public_key: vrf_public_key,
            },
        );
    }

    pub fn deregister_stake_pool(&mut self, pool_id: &StakePoolId) {
        let pool_info = self.stake_pools.remove(pool_id).unwrap();

        // Remove all pool members.
        for member in pool_info.members {
            let stake_key_info = self.stake_keys.get_mut(&member).unwrap();
            assert_eq!(stake_key_info.pool.as_ref().unwrap(), pool_id);
            stake_key_info.pool = None;
        }
    }

    pub fn nr_pool_members(&self, pool_id: StakePoolId) -> usize {
        self.stake_pools[&pool_id].members.len()
    }

    pub fn delegate_stake(&mut self, stake_key_id: StakeKeyId, pool_id: StakePoolId) {
        let stake_key = self.stake_keys.get_mut(&stake_key_id).unwrap();

        // If this is a redelegation, remove the stake key from its previous pool.
        if let Some(prev_stake_pool) = &stake_key.pool {
            let removed = self
                .stake_pools
                .get_mut(&prev_stake_pool)
                .unwrap()
                .members
                .remove(&stake_key_id);
            assert!(removed);
        }

        let stake_pool = self.stake_pools.get_mut(&pool_id).unwrap();
        stake_key.pool = Some(pool_id);
        stake_pool.members.insert(stake_key_id);
    }
}
