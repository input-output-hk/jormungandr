use crate::{
    certificate::{PoolRegistration, PoolPermissions}, leadership::genesis::GenesisPraosLeader, rewards::TaxType,
    testing::data::StakePool,
};
use chain_crypto::{Curve25519_2HashDH, Ed25519, KeyPair, PublicKey, SumEd25519_12};
use chain_time::DurationSeconds;

pub struct StakePoolBuilder {
    owners: Vec<PublicKey<Ed25519>>,
    operators: Vec<PublicKey<Ed25519>>,
    pool_permissions: Option<PoolPermissions>,
    alias: String
}

impl StakePoolBuilder {
    pub fn new() -> Self {
        StakePoolBuilder { 
            owners: Vec::new(), 
            operators: Vec::new(), 
            alias: "".to_owned(), 
            pool_permissions: None 
        }
    }

    pub fn with_owners(&mut self, owners: Vec<PublicKey<Ed25519>>) -> &mut Self {
        self.owners.extend(owners);
        self
    }

    pub fn with_alias(&mut self, alias: &str) -> &mut Self {
        self.alias = alias.to_owned();
        self
    }

    pub fn with_operators(&mut self, operators: Vec<PublicKey<Ed25519>>) -> &mut Self {
        self.operators.extend(operators);
        self
    }

    pub fn with_pool_permissions(&mut self, permissions: PoolPermissions) -> &mut Self {
        self.pool_permissions = Some(permissions);
        self
    }

    pub fn build(&self) -> StakePool {
        let mut rng = rand_os::OsRng::new().unwrap();

        let pool_vrf: KeyPair<Curve25519_2HashDH> = KeyPair::generate(&mut rng);
        let pool_kes: KeyPair<SumEd25519_12> = KeyPair::generate(&mut rng);

        let permissions = match &self.pool_permissions {
            Some(pool_permissions) => pool_permissions.clone(),
            None => PoolPermissions::new(std::cmp::max(self.owners.len() as u8 / 2, 1))
        };

        let pool_info = PoolRegistration {
            serial: 1234,
            owners: self.owners.iter().cloned().collect(),
            operators: self.operators.iter().cloned().collect(),
            start_validity: DurationSeconds::from(0).into(),
            permissions: permissions,
            rewards: TaxType::zero(),
            reward_account: None,
            keys: GenesisPraosLeader {
                vrf_public_key: pool_vrf.public_key().clone(),
                kes_public_key: pool_kes.public_key().clone(),
            },
        };
        StakePool::new(&self.alias,pool_info.to_id(), pool_vrf, pool_kes, pool_info)
    }
}
