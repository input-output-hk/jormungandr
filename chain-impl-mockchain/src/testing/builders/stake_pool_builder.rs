use crate::{
    certificate::{PoolRegistration, PoolPermissions}, leadership::genesis::GenesisPraosLeader, rewards::TaxType,
    testing::data::StakePool,
};
use chain_crypto::{Curve25519_2HashDH, Ed25519, KeyPair, PublicKey, SumEd25519_12};
use chain_time::DurationSeconds;

pub struct StakePoolBuilder {
    owners: Vec<PublicKey<Ed25519>>,
}

impl StakePoolBuilder {
    pub fn new() -> Self {
        StakePoolBuilder { owners: Vec::new() }
    }

    pub fn build(&self) -> StakePool {
        let mut rng = rand_os::OsRng::new().unwrap();

        let pool_vrf: KeyPair<Curve25519_2HashDH> = KeyPair::generate(&mut rng);
        let pool_kes: KeyPair<SumEd25519_12> = KeyPair::generate(&mut rng);

        let pool_info = PoolRegistration {
            serial: 1234,
            owners: self.owners.iter().cloned().collect(),
            operators: vec![].into(),
            start_validity: DurationSeconds::from(0).into(),
            permissions: PoolPermissions::new(std::cmp::max(self.owners.len() as u16 / 2, 1)),
            rewards: TaxType::zero(),
            keys: GenesisPraosLeader {
                vrf_public_key: pool_vrf.public_key().clone(),
                kes_public_key: pool_kes.public_key().clone(),
            },
        };
        StakePool::new(pool_info.to_id(), pool_vrf, pool_kes, pool_info)
    }
}
