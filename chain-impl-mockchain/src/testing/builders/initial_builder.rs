use crate::{
    fragment::Fragment, 
    testing::{
        builders::*,
        data::{
            StakePool,Wallet
        },
        CertificateSigner
    },
    certificate::SignedCertificate,
    transaction::*, 
    key::EitherEd25519SecretKey,
};
use chain_addr::Address;
use std::vec::Vec;

pub fn create_initial_transaction(output: Output<Address>) -> Fragment {
    Fragment::Transaction(TxBuilder::new()
        .set_nopayload()
        .set_ios(&[], &[output])
        .set_witnesses(&[])
        .set_payload_auth(&()))
}

pub fn create_initial_transactions(outputs: &Vec<Output<Address>>) -> Fragment {
    Fragment::Transaction(TxBuilder::new()
        .set_nopayload()
        .set_ios(&[], &outputs)
        .set_witnesses(&[])
        .set_payload_auth(&()))
}

pub fn create_initial_stake_pool_registration(stake_pool: &StakePool, owners: &[Wallet]) -> Fragment {
    let cert = build_stake_pool_registration_cert(&stake_pool.info());
    let keys: Vec<EitherEd25519SecretKey>= owners.iter().cloned().map(|owner| owner.private_key()).collect();
    let signed_cert = CertificateSigner::new().with_certificate(&cert).with_keys(keys).sign().unwrap(); // remove unwrap
    fragment(signed_cert)
}

pub fn create_initial_stake_pool_delegation(stake_pool: &StakePool, wallet: &Wallet) -> Fragment {
    let cert = build_stake_delegation_cert(&stake_pool.info(), &wallet.as_account_data());
    let signed_cert = CertificateSigner::new().with_certificate(&cert).with_key(wallet.private_key().clone()).sign().unwrap(); // remove unwrap
    fragment(signed_cert)
}


fn make_fragment<P: Payload, F>(payload: &P, auth: &P::Auth, to_fragment: F) -> Fragment
   where  F: FnOnce(Transaction<P>) -> Fragment
{
   let tx = TxBuilder::new().set_payload(payload)
        .set_ios(&[], &[])
        .set_witnesses(&[])
        .set_payload_auth(&auth);
    to_fragment(tx)
}

fn fragment(signed_cert: SignedCertificate) -> Fragment {
        match signed_cert.clone().into() {
                SignedCertificate::PoolRegistration(c, a) => {
                    make_fragment(&c, &a, Fragment::PoolRegistration)
                },
                SignedCertificate::PoolUpdate(c, a) => {
                    make_fragment(&c, &a, Fragment::PoolUpdate)
                },
                SignedCertificate::PoolRetirement(c, a) => {
                    make_fragment(&c, &a, Fragment::PoolRetirement)
                },
                SignedCertificate::StakeDelegation(c, a) => {
                    make_fragment(&c, &a, Fragment::StakeDelegation)
                },
                SignedCertificate::OwnerStakeDelegation(c, a) => {
                    make_fragment(&c, &a, Fragment::OwnerStakeDelegation)
                },
        }
}