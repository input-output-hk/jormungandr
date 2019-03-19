use crate::{
    block::Message,
    key::{verify_multi_signature, verify_signature},
};
use chain_crypto::{Ed25519Extended, PublicKey};
use imhamt::{Hamt, UpdateError};
use std::collections::hash_map::DefaultHasher;

use super::role::{StakeKeyId, StakeKeyInfo, StakePoolId, StakePoolInfo};

/// A structure that keeps track of stake keys and stake pools.
#[derive(Clone)]
pub struct DelegationState {
    pub(super) stake_keys: Hamt<DefaultHasher, StakeKeyId, StakeKeyInfo>,
    pub(super) stake_pools: Hamt<DefaultHasher, StakePoolId, StakePoolInfo>,
}

#[derive(Debug, Clone, PartialEq)]
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

    pub(crate) fn apply(&self, message: &Message) -> Result<Self, DelegationError> {
        let mut new_state = self.clone();

        match message {
            Message::StakeDelegation(reg) => {
                if verify_signature(&reg.sig, &reg.data.stake_key_id.0, &reg.data)
                    == chain_crypto::Verification::Failed
                {
                    return Err(DelegationError::StakeDelegationSigIsInvalid);
                }

                if !self.stake_key_exists(&reg.data.stake_key_id) {
                    return Err(DelegationError::StakeDelegationStakeKeyIsInvalid(
                        reg.data.stake_key_id.clone(),
                    ));
                }

                if !self.stake_pool_exists(&reg.data.pool_id) {
                    return Err(DelegationError::StakeDelegationPoolKeyIsInvalid(
                        reg.data.pool_id.clone(),
                    ));
                }

                new_state = new_state
                    .delegate_stake(reg.data.stake_key_id.clone(), reg.data.pool_id.clone())?
            }
            Message::StakeKeyRegistration(reg) => {
                if verify_signature(&reg.sig, &reg.data.stake_key_id.0, &reg.data)
                    == chain_crypto::Verification::Failed
                {
                    return Err(DelegationError::StakeKeyRegistrationSigIsInvalid);
                }

                new_state = new_state.register_stake_key(reg.data.stake_key_id.clone())?
            }
            Message::StakeKeyDeregistration(reg) => {
                if verify_signature(&reg.sig, &reg.data.stake_key_id.0, &reg.data)
                    == chain_crypto::Verification::Failed
                {
                    return Err(DelegationError::StakeKeyDeregistrationSigIsInvalid);
                }

                new_state = new_state.deregister_stake_key(&reg.data.stake_key_id)?
            }
            Message::StakePoolRegistration(reg) => {
                // FIXME verify_multisig
                let owner_keys: Vec<PublicKey<Ed25519Extended>> =
                    reg.data.owners.clone().into_iter().map(|x| x.0).collect();
                if verify_multi_signature(&reg.sig, &owner_keys, &reg.data)
                    == chain_crypto::Verification::Failed
                {
                    return Err(DelegationError::StakePoolRegistrationPoolSigIsInvalid);
                }

                // FIXME: check owner_sig

                // FIXME: should pool_id be a previously registered stake key?

                new_state = new_state.register_stake_pool(reg.data.clone())?
            }
            Message::StakePoolRetirement(reg) => {
                let pool_info = if let Some(pool_info) = self.stake_pools.lookup(&reg.data.pool_id)
                {
                    pool_info
                } else {
                    // TODO: add proper error cause
                    unimplemented!()
                    //return Err(Error::new(ErrorKind::InvalidBlockMessage));
                };

                let owner_keys: Vec<PublicKey<Ed25519Extended>> =
                    pool_info.owners.clone().into_iter().map(|x| x.0).collect();

                if verify_multi_signature(&reg.sig, &owner_keys, &reg.data)
                    == chain_crypto::Verification::Failed
                {
                    return Err(DelegationError::StakePoolRegistrationPoolSigIsInvalid);
                }

                if new_state.stake_pool_exists(&reg.data.pool_id) {
                    // FIXME: support re-registration to change pool parameters.
                    return Err(DelegationError::StakePoolAlreadyExists(
                        reg.data.pool_id.clone(),
                    ));
                }

                // FIXME: check owner_sig

                // FIXME: should pool_id be a previously registered stake key?

                new_state = new_state.deregister_stake_pool(&reg.data.pool_id)?
            }
            Message::Transaction(_) => unreachable!(),
            Message::Update(_) => unreachable!(),
        }

        Ok(new_state)
    }
}
