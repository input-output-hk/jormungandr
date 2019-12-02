#![cfg(test)]

use crate::{
    testing::{
        builders::{StakePoolBuilder,TestTxCertBuilder,build_stake_pool_registration_cert},
        ConfigBuilder, LedgerBuilder,
        data::Wallet,
        TestGen
    },
    ledger::{
        check::{CHECK_POOL_REG_MAXIMUM_OWNERS,CHECK_POOL_REG_MAXIMUM_OPERATORS},
        Error,
    },
    date::BlockDate,
    certificate::{PoolPermissions},
    value::*
};
use std::iter;
use chain_crypto::{Ed25519,PublicKey};

#[test]
pub fn pool_registration_is_accepted() {
    let alice = Wallet::from_value(Value(100));
    let bob = Wallet::from_value(Value(100));
    let clarice = Wallet::from_value(Value(100));

    let mut test_ledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
        .faucets_wallets(vec![&alice,&bob,&clarice])
        .build()
        .expect("cannot build test ledger");

    let stake_pool = StakePoolBuilder::new()
        .with_owners(vec![alice.public_key(),bob.public_key(),clarice.public_key()])
        .with_pool_permissions(PoolPermissions::new(1))
        .build();

    let certificate = build_stake_pool_registration_cert(&stake_pool.info());
    let fragment = TestTxCertBuilder::new(&test_ledger).make_transaction(&vec![&alice,&bob,&clarice], &certificate);
    assert!(test_ledger.apply_fragment(&fragment, BlockDate::first()).is_ok());
}

#[test]
pub fn pool_registration_zero_management_threshold() {
    let alice = Wallet::from_value(Value(100));
    let bob = Wallet::from_value(Value(100));
    let clarice = Wallet::from_value(Value(100));

    let mut test_ledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
        .faucets_wallets(vec![&alice,&bob,&clarice])
        .build()
        .expect("cannot build test ledger");

    let stake_pool = StakePoolBuilder::new()
        .with_owners(vec![alice.public_key(),bob.public_key(),clarice.public_key()])
        .with_pool_permissions(PoolPermissions::new(0))
        .build();

    let certificate = build_stake_pool_registration_cert(&stake_pool.info());
    let fragment = TestTxCertBuilder::new(&test_ledger).make_transaction(&vec![&alice,&bob,&clarice], &certificate);
    assert_err!(
        Error::PoolRegistrationManagementThresholdZero,
        test_ledger.apply_fragment(&fragment, BlockDate::first())
    );
}


#[test]
pub fn pool_registration_management_threshold_above() {
    let alice = Wallet::from_value(Value(100));
    let bob = Wallet::from_value(Value(100));
    let clarice = Wallet::from_value(Value(100));

    let mut test_ledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
        .faucets_wallets(vec![&alice,&bob,&clarice])
        .build()
        .expect("cannot build test ledger");

    let stake_pool = StakePoolBuilder::new()
        .with_owners(vec![alice.public_key(),bob.public_key(),clarice.public_key()])
        .with_pool_permissions(PoolPermissions::new(4))
        .build();

    let certificate = build_stake_pool_registration_cert(&stake_pool.info());
    let fragment = TestTxCertBuilder::new(&test_ledger).make_transaction(&vec![&alice,&bob,&clarice], &certificate);
    assert_err!(
        Error::PoolRegistrationManagementThresholdAbove,
        test_ledger.apply_fragment(&fragment, BlockDate::first())
    );
}

#[test]
pub fn pool_registration_too_many_owners() {
    let alice = Wallet::from_value(Value(100));

    let mut test_ledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
        .faucets_wallets(vec![&alice])
        .build()
        .expect("cannot build test ledger");

    let owners: Vec<PublicKey<Ed25519>> = iter::from_fn(|| Some(TestGen::public_key())).take(CHECK_POOL_REG_MAXIMUM_OWNERS + 1).collect();

    let stake_pool = StakePoolBuilder::new()
        .with_owners(owners)
        .with_pool_permissions(PoolPermissions::new(4))
        .build();

    let certificate = build_stake_pool_registration_cert(&stake_pool.info());
    let fragment = TestTxCertBuilder::new(&test_ledger).make_transaction(&vec![&alice], &certificate);
    assert_err!(
        Error::PoolRegistrationHasTooManyOwners,
        test_ledger.apply_fragment(&fragment, BlockDate::first())
    );
}

#[test]
pub fn pool_registration_too_many_operators() {
    let alice = Wallet::from_value(Value(100));

    let mut test_ledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
        .faucets_wallets(vec![&alice])
        .build()
        .expect("cannot build test ledger");

    let operators: Vec<PublicKey<Ed25519>> = iter::from_fn(|| Some(TestGen::public_key())).take(CHECK_POOL_REG_MAXIMUM_OPERATORS + 1).collect();

    let stake_pool = StakePoolBuilder::new()
        .with_owners(vec![alice.public_key()])
        .with_operators(operators)
        .with_pool_permissions(PoolPermissions::new(1))
        .build();

    let certificate = build_stake_pool_registration_cert(&stake_pool.info());
    let fragment = TestTxCertBuilder::new(&test_ledger).make_transaction(&vec![&alice], &certificate);
    assert_err!(
        Error::PoolRegistrationHasTooManyOperators,
        test_ledger.apply_fragment(&fragment, BlockDate::first())
    );
}

#[test]
#[should_panic]
pub fn pool_registration_zero_signatures() {
    let alice = Wallet::from_value(Value(100));

    let mut test_ledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
        .faucets_wallets(vec![&alice])
        .build()
        .expect("cannot build test ledger");

    let stake_pool = StakePoolBuilder::new()
        .with_owners(vec![alice.public_key()])
        .with_pool_permissions(PoolPermissions::new(1))
        .build();

    let certificate = build_stake_pool_registration_cert(&stake_pool.info());
    let fragment = TestTxCertBuilder::new(&test_ledger).make_transaction_different_signers(&alice,&vec![], &certificate);
    test_ledger.apply_fragment(&fragment, BlockDate::first()).unwrap();
}

#[test]
pub fn pool_registration_too_many_signatures() {
    let alice = Wallet::from_value(Value(100));

    let mut test_ledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
        .faucets_wallets(vec![&alice])
        .build()
        .expect("cannot build test ledger");

    let signers: Vec<Wallet> = iter::from_fn(|| Some(Wallet::from_value(Value(1000)))).take(CHECK_POOL_REG_MAXIMUM_OWNERS + 1).collect();
    let signers: Vec<&Wallet> = signers.iter().map(|x| x).collect();

    let stake_pool = StakePoolBuilder::new()
        .with_owners(vec![alice.public_key()])
        .with_pool_permissions(PoolPermissions::new(1))
        .build();

    let certificate = build_stake_pool_registration_cert(&stake_pool.info());
    let fragment = TestTxCertBuilder::new(&test_ledger).make_transaction_different_signers(&alice,&signers, &certificate);
    assert_err!(
        Error::CertificateInvalidSignature,
        test_ledger.apply_fragment(&fragment, BlockDate::first())
    );
}