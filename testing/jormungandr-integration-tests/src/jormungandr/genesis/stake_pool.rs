use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::{prelude::*, TempDir};
use chain_crypto::{RistrettoGroup2HashDh, SumEd25519_12};
use chain_impl_mockchain::fee::LinearFee;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{Block0ConfigurationBuilder, JormungandrProcess},
    testing::{keys, settings::SettingsDtoExtension},
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{BlockDate, InitialUTxO, Ratio, TaxType, Value},
};
use jortestkit::process::Wait;
use std::str::FromStr;
use thor::Wallet;

#[test]
pub fn create_delegate_retire_stake_pool() {
    let temp_dir = TempDir::new().unwrap();

    let mut actor_account = thor::Wallet::default();

    let config = Block0ConfigurationBuilder::default()
        .with_linear_fees(LinearFee::new(100, 100, 200))
        .with_utxos(vec![InitialUTxO {
            value: 1_000_000.into(),
            address: actor_account.address(),
        }]);

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(config)
        .build()
        .start_node(temp_dir)
        .unwrap();

    let settings = jormungandr.rest().settings().unwrap();

    let stake_pool_id = create_new_stake_pool(
        &mut actor_account,
        settings.genesis_block_hash(),
        BlockDate::new(1, 0),
        &jormungandr,
        &Default::default(),
    );
    delegate_stake(
        &mut actor_account,
        &stake_pool_id,
        settings.genesis_block_hash(),
        BlockDate::new(1, 0),
        &jormungandr,
        &Default::default(),
    );
    retire_stake_pool(
        &stake_pool_id,
        &mut actor_account,
        settings.genesis_block_hash(),
        BlockDate::new(1, 0),
        &jormungandr,
        &Default::default(),
    );
}

pub fn create_new_stake_pool(
    account: &mut Wallet,
    genesis_block_hash: Hash,
    valid_until: BlockDate,
    jormungandr: &JormungandrProcess,
    wait: &Wait,
) -> String {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let kes = keys::create_new_key_pair::<RistrettoGroup2HashDh>();
    let vrf = keys::create_new_key_pair::<SumEd25519_12>();

    let owner_stake_key = temp_dir.child("stake_key.private_key");
    owner_stake_key
        .write_str(&account.signing_key_to_string())
        .unwrap();

    let settings = jcli.rest().v0().settings(jormungandr.rest_uri());
    let fees: LinearFee = settings.fees;
    let fee_value: Value = (fees.certificate + fees.coefficient + fees.constant).into();

    let stake_pool_certificate = jcli.certificate().new_stake_pool_registration(
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
    let stake_pool_certificate_file = temp_dir.child("stake_pool.cert");
    stake_pool_certificate_file
        .write_str(&stake_pool_certificate)
        .unwrap();

    let transaction = jcli
        .transaction_builder(genesis_block_hash)
        .new_transaction()
        .add_account(&account.address().to_string(), &fee_value)
        .add_certificate(&stake_pool_certificate)
        .set_expiry_date(valid_until)
        .finalize_with_fee(&account.address().to_string(), &fees)
        .seal_with_witness_data(account.witness_data())
        .add_auth(owner_stake_key.path())
        .to_message();

    account.confirm_transaction();
    jcli.fragment_sender(jormungandr)
        .send(&transaction)
        .assert_in_block_with_wait(wait);

    let stake_pool_id = jcli
        .certificate()
        .stake_pool_id(stake_pool_certificate_file.path())
        .unwrap();

    assert!(
        jcli.rest()
            .v0()
            .stake_pools(&jormungandr.rest_uri())
            .contains(&stake_pool_id),
        "cannot find stake-pool certificate in blockchain"
    );

    stake_pool_id
}

pub fn delegate_stake(
    account: &mut Wallet,
    stake_pool_id: &str,
    block0_hash: Hash,
    valid_until: BlockDate,
    jormungandr: &JormungandrProcess,
    wait: &Wait,
) {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let owner_stake_key = temp_dir.child("stake_key.private_key");
    owner_stake_key
        .write_str(&account.signing_key_to_string())
        .unwrap();

    let stake_pool_delegation = jcli
        .certificate()
        .new_stake_delegation(stake_pool_id, &account.identifier().to_bech32_str());

    let settings = jcli.rest().v0().settings(&jormungandr.rest_uri());
    let fees: LinearFee = settings.fees;
    let fee_value: Value = (fees.certificate + fees.coefficient + fees.constant).into();

    let transaction = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_account(&account.address().to_string(), &fee_value)
        .add_certificate(&stake_pool_delegation)
        .set_expiry_date(valid_until)
        .finalize_with_fee(&account.address().to_string(), &fees)
        .seal_with_witness_data(account.witness_data())
        .add_auth(owner_stake_key.path())
        .to_message();

    account.confirm_transaction();
    jcli.fragment_sender(jormungandr)
        .send(&transaction)
        .assert_in_block_with_wait(wait);

    let account_state_after_delegation = jcli
        .rest()
        .v0()
        .account_stats(account.address().to_string(), jormungandr.rest_uri());

    let stake_pool_id_hash = Hash::from_str(stake_pool_id).unwrap();
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
    block0_hash: Hash,
    valid_until: BlockDate,
    jormungandr: &JormungandrProcess,
    wait: &Wait,
) {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let owner_stake_key = temp_dir.child("stake_key.private_key");
    owner_stake_key
        .write_str(&account.signing_key_to_string())
        .unwrap();

    let retirement_cert = jcli.certificate().new_stake_pool_retirement(stake_pool_id);

    let settings = jcli.rest().v0().settings(jormungandr.rest_uri());
    let fees: LinearFee = settings.fees;
    let fee_value: Value = (fees.certificate + fees.coefficient + fees.constant).into();

    let transaction = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_account(&account.address().to_string(), &fee_value)
        .add_certificate(&retirement_cert)
        .set_expiry_date(valid_until)
        .finalize_with_fee(&account.address().to_string(), &fees)
        .seal_with_witness_data(account.witness_data())
        .add_auth(owner_stake_key.path())
        .to_message();

    account.confirm_transaction();
    jcli.fragment_sender(jormungandr)
        .send(&transaction)
        .assert_in_block_with_wait(wait);

    let account_state_after_stake_pool_retire = jcli
        .rest()
        .v0()
        .account_stats(account.address().to_string(), jormungandr.rest_uri());

    let stake_pool_id_hash = Hash::from_str(stake_pool_id).unwrap();

    assert!(
        account_state_after_stake_pool_retire
            .delegation()
            .pools()
            .iter()
            .any(|(hash, _)| *hash == stake_pool_id_hash),
        "account should be still delegated to retired pool"
    );

    assert!(
        !jcli
            .rest()
            .v0()
            .stake_pools(&jormungandr.rest_uri())
            .contains(&stake_pool_id.to_owned()),
        "stake pool should not be listed among active stake pools"
    );
}
