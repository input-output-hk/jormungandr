use crate::startup;
use chain_impl_mockchain::{block::BlockDate, testing::TestGen};
use yaml_rust::scanner::TokenType::Value;
use jormungandr_automation::{jcli::JCli,jormungandr::ConfigurationBuilder};

const FIRST_NONCE: u64 = 0;
const MAX_GAS_FEE: u64 = u64::MAX;
const TRANSFER_AMOUNT: u64 = 100;

#[test]
pub fn evm_transaction() {

    let jcli: JCli = Default::default();
    let mut alice = thor::Wallet::default();
    let mut bob = thor::Wallet::default();

    let (jormungandr, _stake_pools) =
        startup::start_stake_pool(&[alice.clone()], &[bob.clone()], &mut ConfigurationBuilder::new())
            .unwrap();

    let alice_account_state_before = jcli
        .rest()
        .v0()
        .account_stats(alice.address().to_string(), jormungandr.rest_uri());
    let bob_account_state_before = jcli
        .rest()
        .v0()
        .account_stats(bob.address().to_string(), &jormungandr.rest_uri());

    println!("Alice balance: {:?}", alice_account_state_before.value());
    println!("Bob balance: {:?}", bob_account_state_before.value());

    let transaction_sender = thor::FragmentSender::from(jormungandr.block0_configuration());

    let fragment_builder = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    );

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
        alice_evm_mapping.evm_address, bob_evm_mapping.evm_address,
        TRANSFER_AMOUNT, MAX_GAS_FEE, FIRST_NONCE);
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

    let alice_balance_after = alice_account_state_after.value();
    let bob_balance_after = bob_account_state_after.value();

    assert_eq!(alice_balance_after, alice_account_state_before.value() - TRANSFER_AMOUNT);
    assert_eq!(bob_balance_after, bob_account_state_before.value() + TRANSFER_AMOUNT);
}