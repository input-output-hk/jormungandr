use crate::wallet::Wallet;
use chain_crypto::{Curve25519_2HashDH, Ed25519, SumEd25519_12};
use chain_impl_mockchain::{
    certificate::{PoolId, PoolPermissions, PoolRegistration},
    rewards::{Ratio as RatioLib, TaxType},
    testing::{builders::StakePoolBuilder, data::StakePool as StakePoolLib},
    value::Value as ValueLib,
};
use jormungandr_lib::crypto::key::KeyPair;
use std::num::NonZeroU64;

#[derive(Clone, Debug)]
pub struct StakePool {
    leader: KeyPair<Ed25519>,
    owner: Wallet,
    inner: StakePoolLib,
}

impl StakePool {
    pub fn new(owner: &Wallet) -> Self {
        let leader = KeyPair::<Ed25519>::generate(rand::rngs::OsRng);

        let stake_pool = StakePoolBuilder::new()
            .with_owners(vec![owner.identifier().into_public_key()])
            .with_pool_permissions(PoolPermissions::new(1))
            .with_reward_account(false)
            .with_tax_type(TaxType {
                fixed: ValueLib(100),
                ratio: RatioLib {
                    numerator: 1,
                    denominator: NonZeroU64::new(10).unwrap(),
                },
                max_limit: None,
            })
            .build();

        StakePool {
            owner: owner.clone(),
            leader,
            inner: stake_pool,
        }
    }

    pub fn leader(&self) -> &KeyPair<Ed25519> {
        &self.leader
    }

    pub fn owner(&self) -> &Wallet {
        &self.owner
    }

    pub fn id(&self) -> PoolId {
        self.inner.id()
    }

    pub fn info_mut(&mut self) -> &mut PoolRegistration {
        self.inner.info_mut()
    }

    pub fn info(&self) -> PoolRegistration {
        self.inner.info()
    }

    pub fn kes(&self) -> KeyPair<SumEd25519_12> {
        KeyPair::<SumEd25519_12>(self.inner.kes())
    }

    pub fn vrf(&self) -> KeyPair<Curve25519_2HashDH> {
        KeyPair::<Curve25519_2HashDH>(self.inner.vrf())
    }
}

impl From<StakePool> for StakePoolLib {
    fn from(stake_pool: StakePool) -> StakePoolLib {
        stake_pool.inner
    }
}
