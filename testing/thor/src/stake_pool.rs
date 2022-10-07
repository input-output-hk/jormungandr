use crate::wallet::Wallet;
use chain_crypto::{RistrettoGroup2HashDh, SumEd25519_12};
use chain_impl_mockchain::{
    certificate::PoolRegistration, testing::data::StakePool as StakePoolLib,
};
use jormungandr_automation::utils::StakePool as InnerStakePool;
use jormungandr_lib::crypto::key::KeyPair;

#[derive(Clone, Debug)]
pub struct StakePool {
    inner: InnerStakePool,
    owner: Wallet,
}

impl StakePool {
    pub fn new(owner: &Wallet) -> Self {
        Self {
            owner: owner.clone(),
            inner: InnerStakePool::new(&owner.identifier()),
        }
    }

    pub fn info_mut(&mut self) -> &mut PoolRegistration {
        self.inner.info_mut()
    }

    pub fn vrf(&self) -> KeyPair<RistrettoGroup2HashDh> {
        self.inner().vrf()
    }

    pub fn kes(&self) -> KeyPair<SumEd25519_12> {
        self.inner().kes()
    }

    pub fn id(&self) -> chain_impl_mockchain::certificate::PoolId {
        self.inner.id()
    }

    pub fn owner(&self) -> &Wallet {
        &self.owner
    }

    pub fn inner(&self) -> &InnerStakePool {
        &self.inner
    }
    pub fn inner_mut(&mut self) -> &mut InnerStakePool {
        &mut self.inner
    }
}

impl From<StakePool> for StakePoolLib {
    fn from(stake_pool: StakePool) -> StakePoolLib {
        stake_pool.inner.into()
    }
}
