use crate::{
    block::Message,
    key::verify_signature,
    leadership::{genesis::GenesisPraosId, Error, ErrorKind},
};
use chain_crypto::{Curve25519_2HashDH, FakeMMM, PublicKey};
use std::collections::{HashMap, HashSet};

use super::role::{StakeKeyId, StakeKeyInfo, StakePoolInfo};

/// A structure that keeps track of stake keys and stake pools.
#[derive(Debug, Clone)]
pub struct DelegationState {
    pub(super) stake_keys: HashMap<StakeKeyId, StakeKeyInfo>,
    pub(super) stake_pools: HashMap<GenesisPraosId, StakePoolInfo>,
}

#[derive(Debug, PartialEq)]
pub enum DelegationError {
    StakeKeyAlreadyRegistered,
    StakeKeyRegistrationSigIsInvalid,
    StakeKeyDeregistrationSigIsInvalid,
    StakeKeyDeregistrationDoesNotExist,
    StakeDelegationSigIsInvalid,
    StakeDelegationStakeKeyIsInvalid(StakeKeyId),
    StakeDelegationPoolKeyIsInvalid(GenesisPraosId),
    StakePoolRegistrationPoolSigIsInvalid,
    StakePoolAlreadyExists(GenesisPraosId),
    StakePoolRetirementSigIsInvalid,
    StakePoolDoesNotExist(GenesisPraosId),
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
    pub fn new(
        initial_stake_pools: Vec<StakePoolInfo>,
        initial_stake_keys: HashMap<StakeKeyId, Option<GenesisPraosId>>,
    ) -> Self {
        let mut stake_pools: HashMap<GenesisPraosId, StakePoolInfo> = initial_stake_pools
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

    pub fn register_stake_key(&mut self, stake_key_id: StakeKeyId) -> Option<StakeKeyInfo> {
        self.stake_keys
            .insert(stake_key_id, StakeKeyInfo { pool: None })
    }

    pub fn deregister_stake_key(&mut self, stake_key_id: &StakeKeyId) -> Option<StakeKeyInfo> {
        let stake_key_info = self.stake_keys.remove(&stake_key_id)?;

        // Remove this stake key from its pool, if any.
        if let Some(ref pool_id) = &stake_key_info.pool {
            self.stake_pools
                .get_mut(pool_id)
                .unwrap()
                .members
                .remove(&stake_key_id);
        }

        Some(stake_key_info)
    }

    pub fn get_stake_pools(&self) -> &HashMap<GenesisPraosId, StakePoolInfo> {
        &self.stake_pools
    }

    pub fn stake_pool_exists(&self, pool_id: &GenesisPraosId) -> bool {
        self.stake_pools.contains_key(pool_id)
    }

    pub fn register_stake_pool(
        &mut self,
        pool_id: GenesisPraosId,
        owner: StakeKeyId,
        kes_public_key: PublicKey<FakeMMM>,
        vrf_public_key: PublicKey<Curve25519_2HashDH>,
    ) {
        assert!(!self.stake_pools.contains_key(&pool_id));
        self.stake_pools.insert(
            pool_id.clone(),
            StakePoolInfo {
                pool_id: pool_id,
                owner: owner,
                members: HashSet::new(),
                kes_public_key: kes_public_key,
                vrf_public_key: vrf_public_key,
            },
        );
    }

    pub fn deregister_stake_pool(&mut self, pool_id: &GenesisPraosId) {
        let pool_info = self.stake_pools.remove(pool_id).unwrap();

        // Remove all pool members.
        for member in pool_info.members {
            let stake_key_info = self.stake_keys.get_mut(&member).unwrap();
            assert_eq!(stake_key_info.pool.as_ref().unwrap(), pool_id);
            stake_key_info.pool = None;
        }
    }

    pub fn nr_pool_members(&self, pool_id: GenesisPraosId) -> usize {
        self.stake_pools[&pool_id].members.len()
    }

    pub fn delegate_stake(&mut self, stake_key_id: StakeKeyId, pool_id: GenesisPraosId) {
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

    pub(crate) fn apply(&self, message: &Message) -> Result<Self, Error> {
        let mut new_state = self.clone();

        match message {
            Message::StakeDelegation(reg) => {
                if verify_signature(&reg.sig, &reg.data.stake_key_id.0, &reg.data)
                    == chain_crypto::Verification::Failed
                {
                    return Err(Error::new_(
                        ErrorKind::InvalidBlockMessage,
                        Box::new(DelegationError::StakeDelegationSigIsInvalid),
                    ));
                }

                if !self.stake_key_exists(&reg.data.stake_key_id) {
                    return Err(Error::new_(
                        ErrorKind::InvalidBlockMessage,
                        Box::new(DelegationError::StakeDelegationStakeKeyIsInvalid(
                            reg.data.stake_key_id.clone(),
                        )),
                    ));
                }

                if !self.stake_pool_exists(&reg.data.pool_id) {
                    return Err(Error::new_(
                        ErrorKind::InvalidBlockMessage,
                        Box::new(DelegationError::StakeDelegationPoolKeyIsInvalid(
                            reg.data.pool_id.clone(),
                        )),
                    ));
                }

                new_state.delegate_stake(reg.data.stake_key_id.clone(), reg.data.pool_id.clone());
            }
            Message::StakeKeyRegistration(reg) => {
                if verify_signature(&reg.sig, &reg.data.stake_key_id.0, &reg.data)
                    == chain_crypto::Verification::Failed
                {
                    return Err(Error::new_(
                        ErrorKind::InvalidBlockMessage,
                        Box::new(DelegationError::StakeKeyRegistrationSigIsInvalid),
                    ));
                }

                if let Some(_original) = new_state.register_stake_key(reg.data.stake_key_id.clone())
                {
                    // FIXME: error stake key already registered
                    return Err(Error::new_(
                        ErrorKind::InvalidBlockMessage,
                        Box::new(DelegationError::StakeKeyAlreadyRegistered),
                    ));
                }
            }
            Message::StakeKeyDeregistration(reg) => {
                if verify_signature(&reg.sig, &reg.data.stake_key_id.0, &reg.data)
                    == chain_crypto::Verification::Failed
                {
                    return Err(Error::new_(
                        ErrorKind::InvalidBlockMessage,
                        Box::new(DelegationError::StakeKeyDeregistrationSigIsInvalid),
                    ));
                }

                if let None = new_state.deregister_stake_key(&reg.data.stake_key_id) {
                    return Err(Error::new_(
                        ErrorKind::InvalidBlockMessage,
                        Box::new(DelegationError::StakeKeyDeregistrationDoesNotExist),
                    ));
                }
            }
            Message::StakePoolRegistration(reg) => {
                if verify_signature(&reg.sig, &reg.data.owner.0, &reg.data)
                    == chain_crypto::Verification::Failed
                {
                    return Err(Error::new_(
                        ErrorKind::InvalidBlockMessage,
                        Box::new(DelegationError::StakePoolRegistrationPoolSigIsInvalid),
                    ));
                }

                if new_state.stake_pool_exists(&reg.data.pool_id) {
                    // FIXME: support re-registration to change pool parameters.
                    return Err(Error::new_(
                        ErrorKind::InvalidBlockMessage,
                        Box::new(DelegationError::StakePoolAlreadyExists(
                            reg.data.pool_id.clone(),
                        )),
                    ));
                }

                // FIXME: check owner_sig

                // FIXME: should pool_id be a previously registered stake key?

                new_state.register_stake_pool(
                    reg.data.pool_id.clone(),
                    reg.data.owner.clone(),
                    reg.data.kes_public_key.clone(),
                    reg.data.vrf_public_key.clone(),
                );
            }
            Message::StakePoolRetirement(reg) => {
                let pool_info = if let Some(pool_info) = self.stake_pools.get(&reg.data.pool_id) {
                    pool_info
                } else {
                    // TODO: add proper error cause
                    return Err(Error::new(ErrorKind::InvalidBlockMessage));
                };

                if verify_signature(&reg.sig, &pool_info.owner.0, &reg.data)
                    == chain_crypto::Verification::Failed
                {
                    return Err(Error::new_(
                        ErrorKind::InvalidBlockMessage,
                        Box::new(DelegationError::StakePoolRegistrationPoolSigIsInvalid),
                    ));
                }

                if new_state.stake_pool_exists(&reg.data.pool_id) {
                    // FIXME: support re-registration to change pool parameters.
                    return Err(Error::new_(
                        ErrorKind::InvalidBlockMessage,
                        Box::new(DelegationError::StakePoolAlreadyExists(
                            reg.data.pool_id.clone(),
                        )),
                    ));
                }

                // FIXME: check owner_sig

                // FIXME: should pool_id be a previously registered stake key?

                new_state.deregister_stake_pool(&reg.data.pool_id);
            }
            Message::Transaction(_) => unreachable!(),
            Message::Update(_) => unreachable!(),
        }

        Ok(new_state)
    }
}
