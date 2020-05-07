use crate::common::{
    file_utils,
    jcli_wrapper::{
        self, certificate::wrapper::JCLICertificateWrapper,
        jcli_transaction_wrapper::JCLITransactionWrapper,
    },
    jormungandr::{ConfigurationBuilder, JormungandrProcess, Starter},
    process_utils::Wait,
    startup,
};

use chain_crypto::{Curve25519_2HashDH, SumEd25519_12};
use chain_impl_mockchain::fee::LinearFee;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{InitialUTxO, Ratio, TaxType, Value},
};
use jormungandr_testing_utils::wallet::Wallet;
use std::str::FromStr;

#[test]
pub fn create_delegate_retire_stake_pool() {
    let mut actor_account = startup::create_new_account_address();

    let config = ConfigurationBuilder::new()
        .with_linear_fees(LinearFee::new(100, 100, 200))
        .with_funds(vec![InitialUTxO {
            value: 1000000.into(),
            address: actor_account.address(),
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let stake_pool_id = create_new_stake_pool(
        &mut actor_account,
        config.genesis_block_hash(),
        &jormungandr,
        &Default::default(),
    );
    delegate_stake(
        &mut actor_account,
        &stake_pool_id,
        config.genesis_block_hash(),
        &jormungandr,
        &Default::default(),
    );
    retire_stake_pool(
        &stake_pool_id,
        &mut actor_account,
        config.genesis_block_hash(),
        &jormungandr,
        &Default::default(),
    );
}

pub fn create_new_stake_pool(
    account: &mut Wallet,
    genesis_block_hash: &str,
    jormungandr: &JormungandrProcess,
    wait: &Wait,
) -> String {
    let kes = startup::create_new_key_pair::<Curve25519_2HashDH>();
    let vrf = startup::create_new_key_pair::<SumEd25519_12>();

    let owner_stake_key =
        file_utils::create_file_in_temp("stake_key.private_key", &account.signing_key_as_str());

    let settings = jcli_wrapper::assert_get_rest_settings(&jormungandr.rest_address());
    let fees: LinearFee = settings.fees.into();
    let fee_value: Value = (fees.certificate + fees.coefficient + fees.constant).into();

    let certificate_wrapper = JCLICertificateWrapper::new();

    let stake_pool_certificate = certificate_wrapper.assert_new_stake_pool_registration(
        &vrf.identifier().to_bech32_str(),
        &kes.identifier().to_bech32_str(),
        0u32,
        1u32,
        &account.identifier().to_bech32_str(),
        Some(TaxType {
            fixed: 0.into(),
            ratio: Ratio::new_checked(1, 2).unwrap(),
            max_limit: None,
        }),
    );
    let stake_pool_certificate_file =
        file_utils::create_file_in_temp("stake_pool.cert", &stake_pool_certificate);

    let transaction = JCLITransactionWrapper::new_transaction(genesis_block_hash)
        .assert_add_account(&account.address().to_string(), &fee_value)
        .assert_add_certificate(&stake_pool_certificate)
        .assert_finalize_with_fee(&account.address().to_string(), &fees)
        .seal_with_witness_for_address(account)
        .assert_add_auth(&owner_stake_key)
        .assert_to_message();

    account.confirm_transaction();
    jcli_wrapper::assert_transaction_in_block_with_wait(&transaction, &jormungandr, wait);

    let stake_pool_id = certificate_wrapper.assert_get_stake_pool_id(&stake_pool_certificate_file);

    assert!(
        jcli_wrapper::assert_rest_get_stake_pools(&jormungandr.rest_address())
            .contains(&stake_pool_id),
        "cannot find stake-pool certificate in blockchain"
    );

    stake_pool_id.to_owned()
}

pub fn delegate_stake(
    account: &mut Wallet,
    stake_pool_id: &str,
    genesis_block_hash: &str,
    jormungandr: &JormungandrProcess,
    wait: &Wait,
) {
    let owner_stake_key =
        file_utils::create_file_in_temp("stake_key.private_key", &account.signing_key_as_str());
    let certificate_wrapper = JCLICertificateWrapper::new();
    let stake_pool_delegation = certificate_wrapper
        .assert_new_stake_delegation(&stake_pool_id, &account.identifier().to_bech32_str());

    let settings = jcli_wrapper::assert_get_rest_settings(&jormungandr.rest_address());
    let fees: LinearFee = settings.fees.into();
    let fee_value: Value = (fees.certificate + fees.coefficient + fees.constant).into();

    let transaction = JCLITransactionWrapper::new_transaction(genesis_block_hash)
        .assert_add_account(&account.address().to_string(), &fee_value)
        .assert_add_certificate(&stake_pool_delegation)
        .assert_finalize_with_fee(&account.address().to_string(), &fees)
        .seal_with_witness_for_address(account)
        .assert_add_auth(&owner_stake_key)
        .assert_to_message();

    account.confirm_transaction();
    jcli_wrapper::assert_transaction_in_block_with_wait(&transaction, &jormungandr, wait);

    let account_state_after_delegation = jcli_wrapper::assert_rest_account_get_stats(
        &account.address().to_string(),
        &jormungandr.rest_address(),
    );

    let stake_pool_id_hash = Hash::from_str(&stake_pool_id).unwrap();
    assert!(
        account_state_after_delegation
            .delegation()
            .pools()
            .iter()
            .any(|(hash, _)| *hash == stake_pool_id_hash),
        "account should be delegated to pool"
    );
}

pub fn retire_stake_pool(
    stake_pool_id: &str,
    account: &mut Wallet,
    genesis_block_hash: &str,
    jormungandr: &JormungandrProcess,
    wait: &Wait,
) {
    let owner_private_key =
        file_utils::create_file_in_temp("stake_key.private_key", &account.signing_key_as_str());

    let certificate_wrapper = JCLICertificateWrapper::new();

    let retirement_cert = certificate_wrapper.assert_new_stake_pool_retirement(&stake_pool_id);

    let settings = jcli_wrapper::assert_get_rest_settings(&jormungandr.rest_address());
    let fees: LinearFee = settings.fees.into();
    let fee_value: Value = (fees.certificate + fees.coefficient + fees.constant).into();

    let transaction = JCLITransactionWrapper::new_transaction(genesis_block_hash)
        .assert_add_account(&account.address().to_string(), &fee_value)
        .assert_add_certificate(&retirement_cert)
        .assert_finalize_with_fee(&account.address().to_string(), &fees)
        .seal_with_witness_for_address(&account)
        .assert_add_auth(&owner_private_key)
        .assert_to_message();

    account.confirm_transaction();
    jcli_wrapper::assert_transaction_in_block_with_wait(&transaction, &jormungandr, wait);

    let account_state_after_stake_pool_retire = jcli_wrapper::assert_rest_account_get_stats(
        &account.address().to_string(),
        &jormungandr.rest_address(),
    );

    let stake_pool_id_hash = Hash::from_str(&stake_pool_id).unwrap();

    assert!(
        account_state_after_stake_pool_retire
            .delegation()
            .pools()
            .iter()
            .any(|(hash, _)| *hash == stake_pool_id_hash),
        "account should be still delegated to retired pool"
    );

    assert!(
        !jcli_wrapper::assert_rest_get_stake_pools(&jormungandr.rest_address())
            .contains(&stake_pool_id.to_owned()),
        "stake pool should not be listed among active stake pools"
    );
}
