use chain_crypto::{Ed25519, RistrettoGroup2HashDh, SumEd25519_12};
use chain_impl_mockchain::{
    certificate::{PoolId, PoolPermissions, PoolRegistration},
    rewards::{Ratio as RatioLib, TaxType},
    testing::{builders::StakePoolBuilder, data::StakePool as StakePoolLib},
    value::Value as ValueLib,
};
use jormungandr_lib::crypto::key::{Identifier, KeyPair};
use std::num::NonZeroU64;

#[derive(Clone, Debug)]
pub struct StakePool {
    leader: KeyPair<Ed25519>,
    owner: Identifier<Ed25519>,
    inner: StakePoolLib,
}

impl StakePool {
    pub fn new(owner_identifier: &Identifier<Ed25519>) -> Self {
        let leader = KeyPair::<Ed25519>::generate(rand::rngs::OsRng);

        let stake_pool = StakePoolBuilder::new()
            .with_owners(vec![owner_identifier.clone().into_public_key()])
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
            owner: owner_identifier.clone(),
            leader,
            inner: stake_pool,
        }
    }

    pub fn leader(&self) -> &KeyPair<Ed25519> {
        &self.leader
    }

    pub fn owner(&self) -> &Identifier<Ed25519> {
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

    pub fn vrf(&self) -> KeyPair<RistrettoGroup2HashDh> {
        KeyPair::<RistrettoGroup2HashDh>(self.inner.vrf())
    }
}

impl From<StakePool> for StakePoolLib {
    fn from(stake_pool: StakePool) -> StakePoolLib {
        stake_pool.inner
    }
}
