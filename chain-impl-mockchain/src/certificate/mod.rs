mod delegation;
mod pool;

#[cfg(test)]
mod test;

pub use delegation::{OwnerStakeDelegation, StakeDelegation};
pub use pool::{
    PoolId, PoolManagement, PoolOwnersSigned, PoolRegistration, PoolRetirement, PoolUpdate,
};

#[derive(Debug, Clone)]
pub enum Certificate {
    StakeDelegation(StakeDelegation),
    OwnerStakeDelegation(OwnerStakeDelegation),
    PoolRegistration(PoolRegistration),
    PoolManagement(PoolManagement),
}
