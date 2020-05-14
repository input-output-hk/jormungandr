pub use self::{
    initial_certificates::{signed_delegation_cert, signed_stake_pool_cert},
    transaction::transaction_to,
};
use crate::wallet::Wallet;
use chain_impl_mockchain::{
    certificate::PoolId,
    fee::LinearFee,
    fragment::Fragment,
    testing::{
        data::{StakePool, Wallet as WalletLib},
        scenario::FragmentFactory,
    },
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Address, Initial, Value},
};
use thiserror::Error;

mod initial_certificates;
mod transaction;

#[derive(Error, Debug)]
pub enum FragmentBuilderError {
    #[error("cannot compute the transaction's balance")]
    CannotComputeBalance,
    #[error("Cannot compute the new fees of {0} for a new input")]
    CannotAddCostOfExtraInput(u64),
    #[error("transaction already balanced")]
    TransactionAlreadyBalanced,
    #[error("the transaction has {0} value extra than necessary")]
    TransactionAlreadyExtraValue(Value),
}

pub struct FragmentBuilder {
    block0_hash: Hash,
    fees: LinearFee,
}

impl FragmentBuilder {
    pub fn new(block0_hash: &Hash, fees: &LinearFee) -> Self {
        Self {
            block0_hash: block0_hash.clone(),
            fees: fees.clone(),
        }
    }

    fn fragment_factory(&self) -> FragmentFactory {
        FragmentFactory::new(self.block0_hash.into_hash(), self.fees)
    }

    pub fn transaction(
        &self,
        from: &Wallet,
        address: Address,
        value: Value,
    ) -> Result<Fragment, FragmentBuilderError> {
        transaction_to(
            &self.block0_hash,
            &self.fees,
            &from.clone().into(),
            address,
            value,
        )
    }

    pub fn full_delegation_cert_for_block0(wallet: &Wallet, pool_id: PoolId) -> Initial {
        Initial::Cert(signed_delegation_cert(wallet, pool_id).into())
    }

    pub fn stake_pool_registration(&self, funder: &Wallet, stake_pool: &StakePool) -> Fragment {
        let mut inner_wallet = funder.clone().into();
        self.fragment_factory()
            .stake_pool_registration(&mut inner_wallet, stake_pool)
    }

    pub fn delegation(&self, from: &Wallet, stake_pool: &StakePool) -> Fragment {
        let mut inner_wallet = from.clone().into();
        self.fragment_factory()
            .delegation(&mut inner_wallet, stake_pool)
    }

    pub fn delegation_remove(&self, from: &Wallet) -> Fragment {
        let mut inner_wallet = from.clone().into();
        self.fragment_factory().delegation_remove(&mut inner_wallet)
    }

    pub fn delegation_to_many(&self, from: &Wallet, distribution: &[(&StakePool, u8)]) -> Fragment {
        let mut inner_wallet = from.clone().into();
        self.fragment_factory()
            .delegation_to_many(&mut inner_wallet, distribution)
    }

    pub fn owner_delegation(&self, from: &Wallet, stake_pool: &StakePool) -> Fragment {
        let mut inner_wallet = from.clone().into();
        self.fragment_factory()
            .owner_delegation(&mut inner_wallet, stake_pool)
    }

    pub fn stake_pool_retire(&self, owners: Vec<&Wallet>, stake_pool: &StakePool) -> Fragment {
        let inner_owners: Vec<WalletLib> = owners
            .iter()
            .cloned()
            .map(|x| {
                let wallet: WalletLib = x.clone().into();
                wallet
            })
            .collect();

        let ref_inner_owners: Vec<&WalletLib> = inner_owners.iter().collect();
        self.fragment_factory()
            .stake_pool_retire(&ref_inner_owners[..], stake_pool)
    }
}
