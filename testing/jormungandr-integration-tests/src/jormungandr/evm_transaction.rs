use crate::{startup, startup::SingleNodeTestBootstrapper};
use assert_fs::fixture::TempDir;
use chain_impl_mockchain::{block::BlockDate, testing::TestGen};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{Block0ConfigurationBuilder, NodeConfigBuilder},
};
use jormungandr_lib::interfaces::InitialUTxO;

const FIRST_NONCE: u64 = 0;
const WRONG_NONCE: u64 = u64::MAX;
const MAX_GAS_FEE: u64 = u64::MAX;
const TRANSFER_AMOUNT: u64 = 100;
const INITIAL_BALANCE: u64 = 1000;
const INSUFFICIENT_FUNDS_INITIAL_BALANCE: u64 = 1;

#[test]
pub fn evm_transaction() {
    let jcli: JCli = Default::default();
    let mut alice = thor::Wallet::default();
    let mut bob = thor::Wallet::default();

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[bob.clone()],
        Block0ConfigurationBuilder::default(),
        NodeConfigBuilder::default(),
    )
    .unwrap();

    let alice_account_state_before = jcli
        .rest()
        .v0()
        .account_stats(alice.address().to_string(), jormungandr.rest_uri());
    let bob_account_state_before = jcli
        .rest()
        .v0()
        .account_stats(bob.address().to_string(), &jormungandr.rest_uri());

    let alice_account_balance_before: u64 = (*alice_account_state_before.value()).into();
    let bob_account_balance_before: u64 = (*bob_account_state_before.value()).into();

    let settings = jormungandr.rest().settings().unwrap();

    let transaction_sender = thor::FragmentSender::from(&settings);

    let fragment_builder =
        thor::FragmentBuilder::from_settings(&settings, BlockDate::first().next_epoch());

    let alice_evm_mapping = TestGen::evm_mapping_for_wallet(&alice.clone().into());
    let alice_mapping_fragment = fragment_builder.evm_mapping(&alice, &alice_evm_mapping);

    let bob_evm_mapping = TestGen::evm_mapping_for_wallet(&bob.clone().into());
    let bob_mapping_fragment = fragment_builder.evm_mapping(&bob, &bob_evm_mapping);

    transaction_sender
        .send_fragment(&mut alice, alice_mapping_fragment, &jormungandr)
        .unwrap();

    transaction_sender
        .send_fragment(&mut bob, bob_mapping_fragment, &jormungandr)
        .unwrap();

    alice.confirm_transaction();
    bob.confirm_transaction();

    let evm_transaction = TestGen::evm_transaction(
        alice_evm_mapping.evm_address,
        bob_evm_mapping.evm_address,
        TRANSFER_AMOUNT,
        MAX_GAS_FEE,
        FIRST_NONCE,
    );
    let evm_transaction_fragment = fragment_builder.evm_transaction(evm_transaction);

    transaction_sender
        .send_fragment(&mut alice, evm_transaction_fragment, &jormungandr)
        .unwrap();

    alice.confirm_transaction();

    let alice_account_state_after = jcli
        .rest()
        .v0()
        .account_stats(alice.address().to_string(), jormungandr.rest_uri());
    let bob_account_state_after = jcli
        .rest()
        .v0()
        .account_stats(bob.address().to_string(), &jormungandr.rest_uri());

    let alice_balance_after: u64 = (*alice_account_state_after.value()).into();
    let bob_balance_after: u64 = (*bob_account_state_after.value()).into();

    assert_eq!(
        alice_balance_after,
        alice_account_balance_before - TRANSFER_AMOUNT
    );
    assert_eq!(
        bob_balance_after,
        bob_account_balance_before + TRANSFER_AMOUNT
    );
}

#[test]
pub fn evm_transaction_wrong_nonce() {
    let mut alice = thor::Wallet::default();
    let mut bob = thor::Wallet::default();

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[bob.clone()],
        Block0ConfigurationBuilder::default(),
        NodeConfigBuilder::default(),
    )
    .unwrap();

    let settings = jormungandr.rest().settings().unwrap();

    let transaction_sender = thor::FragmentSender::from(&settings);

    let fragment_builder =
        thor::FragmentBuilder::from_settings(&settings, BlockDate::first().next_epoch());

    let alice_evm_mapping = TestGen::evm_mapping_for_wallet(&alice.clone().into());
    let alice_mapping_fragment = fragment_builder.evm_mapping(&alice, &alice_evm_mapping);

    let bob_evm_mapping = TestGen::evm_mapping_for_wallet(&bob.clone().into());
    let bob_mapping_fragment = fragment_builder.evm_mapping(&bob, &bob_evm_mapping);

    transaction_sender
        .send_fragment(&mut alice, alice_mapping_fragment, &jormungandr)
        .unwrap();

    transaction_sender
        .send_fragment(&mut bob, bob_mapping_fragment, &jormungandr)
        .unwrap();

    alice.confirm_transaction();
    bob.confirm_transaction();

    let evm_transaction = TestGen::evm_transaction(
        alice_evm_mapping.evm_address,
        bob_evm_mapping.evm_address,
        TRANSFER_AMOUNT,
        MAX_GAS_FEE,
        WRONG_NONCE,
    );
    let evm_transaction_fragment = fragment_builder.evm_transaction(evm_transaction);

    assert!(
        transaction_sender
            .send_fragment(&mut alice, evm_transaction_fragment, &jormungandr)
            .is_err(),
        "Sending evm transaction with wrong nonce did not fail as expected."
    );
}

#[test]
pub fn evm_transaction_insufficient_funds() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();
    let mut alice = thor::Wallet::default();
    let mut bob = thor::Wallet::default();

    let config = Block0ConfigurationBuilder::default().with_utxos(vec![
        InitialUTxO {
            address: alice.address(),
            value: INSUFFICIENT_FUNDS_INITIAL_BALANCE.into(),
        },
        InitialUTxO {
            address: bob.address(),
            value: INITIAL_BALANCE.into(),
        },
    ]);

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(config)
        .build()
        .start_node(temp_dir)
        .unwrap();

    let alice_account_state_before = jcli
        .rest()
        .v0()
        .account_stats(alice.address().to_string(), jormungandr.rest_uri());
    let bob_account_state_before = jcli
        .rest()
        .v0()
        .account_stats(bob.address().to_string(), &jormungandr.rest_uri());

    let alice_account_balance_before: u64 = (*alice_account_state_before.value()).into();
    let bob_account_balance_before: u64 = (*bob_account_state_before.value()).into();

    let settings = jormungandr.rest().settings().unwrap();

    let transaction_sender = thor::FragmentSender::from(&settings);

    let fragment_builder =
        thor::FragmentBuilder::from_settings(&settings, BlockDate::first().next_epoch());

    let alice_evm_mapping = TestGen::evm_mapping_for_wallet(&alice.clone().into());
    let alice_mapping_fragment = fragment_builder.evm_mapping(&alice, &alice_evm_mapping);

    let bob_evm_mapping = TestGen::evm_mapping_for_wallet(&bob.clone().into());
    let bob_mapping_fragment = fragment_builder.evm_mapping(&bob, &bob_evm_mapping);

    transaction_sender
        .send_fragment(&mut alice, alice_mapping_fragment, &jormungandr)
        .unwrap();

    transaction_sender
        .send_fragment(&mut bob, bob_mapping_fragment, &jormungandr)
        .unwrap();

    alice.confirm_transaction();
    bob.confirm_transaction();

    let evm_transaction = TestGen::evm_transaction(
        alice_evm_mapping.evm_address,
        bob_evm_mapping.evm_address,
        TRANSFER_AMOUNT,
        MAX_GAS_FEE,
        FIRST_NONCE,
    );
    let evm_transaction_fragment = fragment_builder.evm_transaction(evm_transaction);

    assert!(
        transaction_sender
            .send_fragment(&mut alice, evm_transaction_fragment, &jormungandr)
            .is_err(),
        "Sending evm transaction with insufficient funds did not fail as expected."
    );

    let alice_account_state_after = jcli
        .rest()
        .v0()
        .account_stats(alice.address().to_string(), jormungandr.rest_uri());
    let bob_account_state_after = jcli
        .rest()
        .v0()
        .account_stats(bob.address().to_string(), &jormungandr.rest_uri());

    let alice_balance_after: u64 = (*alice_account_state_after.value()).into();
    let bob_balance_after: u64 = (*bob_account_state_after.value()).into();

    assert_eq!(alice_balance_after, alice_account_balance_before);

    assert_eq!(bob_balance_after, bob_account_balance_before);
}
