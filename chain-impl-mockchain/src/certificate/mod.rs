mod delegation;
mod pool;

#[cfg(test)]
mod test;

use crate::transaction::Payload;

pub use delegation::{OwnerStakeDelegation, StakeDelegation};
pub use pool::{
    IndexSignatures, PoolId, PoolOwnersSigned, PoolRegistration, PoolRetirement, PoolUpdate,
};

#[derive(Debug, Clone)]
pub enum Certificate {
    StakeDelegation(StakeDelegation),
    OwnerStakeDelegation(OwnerStakeDelegation),
    PoolRegistration(PoolRegistration),
    PoolRetirement(PoolRetirement),
    PoolUpdate(PoolUpdate),
}

impl Certificate {
    pub fn need_auth(&self) -> bool {
        match self {
            Certificate::PoolRegistration(_) => <PoolRegistration as Payload>::HAS_AUTH,
            Certificate::PoolUpdate(_) => <PoolUpdate as Payload>::HAS_AUTH,
            Certificate::PoolRetirement(_) => <PoolRetirement as Payload>::HAS_AUTH,
            Certificate::StakeDelegation(_) => <StakeDelegation as Payload>::HAS_AUTH,
            Certificate::OwnerStakeDelegation(_) => <OwnerStakeDelegation as Payload>::HAS_AUTH,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SignedCertificate {
    StakeDelegation(StakeDelegation, <StakeDelegation as Payload>::Auth),
    OwnerStakeDelegation(
        OwnerStakeDelegation,
        <OwnerStakeDelegation as Payload>::Auth,
    ),
    PoolRegistration(PoolRegistration, <PoolRegistration as Payload>::Auth),
    PoolRetirement(PoolRetirement, <PoolRetirement as Payload>::Auth),
    PoolUpdate(PoolUpdate, <PoolUpdate as Payload>::Auth),
}
