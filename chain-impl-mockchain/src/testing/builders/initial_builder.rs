use crate::{
    fragment::Fragment, 
    testing::{
        builders::*,
        data::{
            StakePool,Wallet
        },
    },
    transaction::*, 
};
use chain_addr::Address;
use std::vec::Vec;

pub fn create_initial_transaction(output: Output<Address>) -> Fragment {
    let mut builder = TransactionBuilder::new();
    let authenticator = builder.with_output(output).authenticate();
    authenticator.as_message()
}

pub fn create_initial_transactions(outputs: &Vec<Output<Address>>) -> Fragment {
    let mut builder = TransactionBuilder::new();
    let authenticator = builder.with_outputs(outputs.to_vec()).authenticate();
    authenticator.as_message()
}

pub fn create_initial_stake_pool_registrations(stake_pools: &Vec<StakePool> ) ->  Vec<Fragment> {
    stake_pools.iter().map(|x| create_initial_stake_pool_registration(&x)).collect()
}

pub fn create_initial_stake_pool_registration(stake_pool: &StakePool) -> Fragment {
    let cert = build_stake_pool_registration_cert(&stake_pool.info());
    TransactionCertBuilder::new()
            .with_certificate(cert)
            .authenticate()
            .as_message()
}

pub fn create_initial_stake_pool_delegation(stake_pool: &StakePool, wallet: &Wallet) -> Fragment {
    let cert = build_stake_delegation_cert(&stake_pool.info(), &wallet.as_account_data());
    TransactionCertBuilder::new()
        .with_certificate(cert)
        .authenticate()
        .as_message()
}