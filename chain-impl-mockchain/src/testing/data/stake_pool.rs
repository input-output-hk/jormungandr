use crate::certificate::{PoolId, PoolRegistration};
use chain_crypto::{Curve25519_2HashDH, KeyPair, SecretKey, SumEd25519_12};

#[derive(Clone, Debug)]
pub struct StakePool {
    id: PoolId,
    vrf: KeyPair<Curve25519_2HashDH>,
    kes: KeyPair<SumEd25519_12>,
    pool_info: PoolRegistration,
}

impl StakePool {
    pub fn new(
        id: PoolId,
        vrf: KeyPair<Curve25519_2HashDH>,
        kes: KeyPair<SumEd25519_12>,
        pool_info: PoolRegistration,
    ) -> Self {
        StakePool {
            id,
            vrf,
            kes,
            pool_info,
        }
    }

    pub fn id(&self) -> PoolId {
        self.id.clone()
    }

    pub fn vrf(&self) -> KeyPair<Curve25519_2HashDH> {
        self.vrf.clone()
    }

    pub fn kes(&self) -> KeyPair<SumEd25519_12> {
        self.kes.clone()
    }

    pub fn info(&self) -> PoolRegistration {
        self.pool_info.clone()
    }
}