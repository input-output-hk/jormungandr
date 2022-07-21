use crate::startup;
use chain_impl_mockchain::{block::BlockDate, testing::TestGen};
use jormungandr_automation::jormungandr::ConfigurationBuilder;

#[test]
pub fn test_evm_mapping() {
    let mut alice = thor::Wallet::default();

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[alice.clone()],
        &mut ConfigurationBuilder::new(),
    )
    .unwrap();

    let transaction_sender = thor::FragmentSender::from(jormungandr.block0_configuration());

    let fragment_builder = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    );

    let evm_mapping = TestGen::evm_mapping_for_wallet(&alice.clone().into());

    assert_eq!(
        "null",
        jormungandr
            .rest()
            .raw()
            .evm_address(evm_mapping.account_id())
            .unwrap()
            .text()
            .unwrap(),
        "Evm address already existing"
    );

    let alice_fragment = fragment_builder.evm_mapping(&alice, &evm_mapping);

    transaction_sender
        .send_fragment(&mut alice, alice_fragment, &jormungandr)
        .unwrap();

    assert_eq!(
        evm_mapping.evm_address().to_string(),
        jormungandr
            .rest()
            .evm_address(evm_mapping.account_id())
            .unwrap(),
        "Evm address not equal"
    );

    assert_eq!(
        evm_mapping.account_id().to_string(),
        jormungandr
            .rest()
            .jor_address(evm_mapping.evm_address())
            .unwrap(),
        "Jor address not equal"
    );
}

#[test]
pub fn test_evm_mapping_twice() {
    let mut alice = thor::Wallet::default();
    let mut bob = thor::Wallet::default();

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[bob.clone()],
        &mut ConfigurationBuilder::new(),
    )
    .unwrap();

    let transaction_sender = thor::FragmentSender::from(jormungandr.block0_configuration());

    let fragment_builder = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    );

    let evm_mapping_alice = TestGen::evm_mapping_for_wallet(&alice.clone().into());

    assert_eq!(
        "null",
        jormungandr
            .rest()
            .raw()
            .evm_address(evm_mapping_alice.account_id())
            .unwrap()
            .text()
            .unwrap(),
        "Evm address already existing"
    );

    let alice_fragment = fragment_builder.evm_mapping(&alice, &evm_mapping_alice);

    transaction_sender
        .send_fragment(&mut alice, alice_fragment, &jormungandr)
        .unwrap();

    assert_eq!(
        evm_mapping_alice.evm_address().to_string(),
        jormungandr
            .rest()
            .evm_address(evm_mapping_alice.account_id())
            .unwrap(),
        "Evm address not equal"
    );

    assert_eq!(
        evm_mapping_alice.account_id().to_string(),
        jormungandr
            .rest()
            .jor_address(evm_mapping_alice.evm_address())
            .unwrap(),
        "Jor address not equal"
    );

    let evm_mapping_bob = TestGen::evm_mapping_for_wallet(&bob.clone().into());

    assert_eq!(
        "null",
        jormungandr
            .rest()
            .raw()
            .evm_address(evm_mapping_bob.account_id())
            .unwrap()
            .text()
            .unwrap(),
        "Evm address already existing"
    );

    let bob_fragment = fragment_builder.evm_mapping(&bob, &evm_mapping_bob);

    transaction_sender
        .send_fragment(&mut bob, bob_fragment, &jormungandr)
        .unwrap();

    assert_eq!(
        evm_mapping_bob.evm_address().to_string(),
        jormungandr
            .rest()
            .evm_address(evm_mapping_bob.account_id())
            .unwrap(),
        "Evm address not equal"
    );

    assert_eq!(
        evm_mapping_bob.account_id().to_string(),
        jormungandr
            .rest()
            .jor_address(evm_mapping_bob.evm_address())
            .unwrap(),
        "Jor address not equal"
    );
}

#[test]
pub fn test_evm_mapping_already_mapped() {
    let mut alice = thor::Wallet::default();

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[alice.clone()],
        &mut ConfigurationBuilder::new(),
    )
    .unwrap();

    let transaction_sender = thor::FragmentSender::from(jormungandr.block0_configuration());

    let fragment_builder = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    );

    let evm_mapping = TestGen::evm_mapping_for_wallet(&alice.clone().into());

    assert_eq!(
        "null",
        jormungandr
            .rest()
            .raw()
            .evm_address(evm_mapping.account_id())
            .unwrap()
            .text()
            .unwrap(),
        "Evm address already existing"
    );

    let alice_fragment = fragment_builder.evm_mapping(&alice, &evm_mapping);

    transaction_sender
        .send_fragment(&mut alice, alice_fragment, &jormungandr)
        .unwrap();

    assert_eq!(
        evm_mapping.evm_address().to_string(),
        jormungandr
            .rest()
            .evm_address(evm_mapping.account_id())
            .unwrap(),
        "Evm address not equal"
    );

    assert_eq!(
        evm_mapping.account_id().to_string(),
        jormungandr
            .rest()
            .jor_address(evm_mapping.evm_address())
            .unwrap(),
        "Jor address not equal"
    );

    let evm_mapping_2 = TestGen::evm_mapping_for_wallet(&alice.clone().into());

    let alice_fragment_2 = fragment_builder.evm_mapping(&alice, &evm_mapping_2);

    transaction_sender
        .send_fragment(&mut alice, alice_fragment_2, &jormungandr)
        .unwrap_err();

    assert_eq!(
        evm_mapping.evm_address().to_string(),
        jormungandr
            .rest()
            .evm_address(evm_mapping.account_id())
            .unwrap(),
        "Evm address not equal"
    );

    assert_eq!(
        evm_mapping.account_id().to_string(),
        jormungandr
            .rest()
            .jor_address(evm_mapping.evm_address())
            .unwrap(),
        "Jor address not equal"
    );
}
