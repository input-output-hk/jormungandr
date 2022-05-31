use crate::startup;
use chain_impl_mockchain::block::BlockDate;
use chain_impl_mockchain::testing::TestGen;
use jormungandr_automation::jcli::JCli;
use jormungandr_automation::jormungandr::ConfigurationBuilder;

#[test]
pub fn test_evm_mapping() {
    let mut alice = thor::Wallet::default();
    let bob = thor::Wallet::default();

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

    let evm_mapping = TestGen::evm_mapping_for_wallet(&alice.clone().into());

    jormungandr
        .rest()
        .evm_address(&evm_mapping.account_id().to_string())
        .unwrap_err();

    jormungandr
        .rest()
        .jor_address(&evm_mapping.evm_address().to_string())
        .unwrap_err();

    let alice_fragment = fragment_builder.evm_mapping(&alice, &evm_mapping);

    transaction_sender
        .send_fragment(&mut alice, alice_fragment.clone(), &jormungandr)
        .unwrap();

    assert_eq!(
        evm_mapping.account_id().to_string(),
        jormungandr
            .rest()
            .jor_address(&evm_mapping.evm_address().to_string())
            .unwrap()
    );

    assert_eq!(
        evm_mapping.evm_address().to_string(),
        jormungandr
            .rest()
            .evm_address(&evm_mapping.account_id().to_string())
            .unwrap()
    );
}

pub fn test_evm_mapping_already_mapped() {
    let mut alice = thor::Wallet::default();
    let bob = thor::Wallet::default();

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

    let evm_mapping = TestGen::evm_mapping_for_wallet(&alice.clone().into());

    jormungandr
        .rest()
        .evm_address(&evm_mapping.account_id().to_string())
        .unwrap_err();

    jormungandr
        .rest()
        .jor_address(&evm_mapping.evm_address().to_string())
        .unwrap_err();

    let alice_fragment = fragment_builder.evm_mapping(&alice, &evm_mapping);

    transaction_sender
        .send_fragment(&mut alice, alice_fragment.clone(), &jormungandr)
        .unwrap();

    assert_eq!(
        evm_mapping.evm_address().to_string(),
        jormungandr
            .rest()
            .evm_address(&evm_mapping.account_id().to_string())
            .unwrap()
    );

    assert_eq!(
        evm_mapping.account_id().to_string(),
        jormungandr
            .rest()
            .jor_address(&evm_mapping.evm_address().to_string())
            .unwrap()
    );

    let evm_mapping_2 = TestGen::evm_mapping_for_wallet(&alice.clone().into());

    let alice_fragment_2 = fragment_builder.evm_mapping(&alice, &evm_mapping_2);

    transaction_sender
        .send_fragment(&mut alice, alice_fragment_2.clone(), &jormungandr)
        .unwrap();

    assert_eq!(
        evm_mapping.evm_address().to_string(),
        jormungandr
            .rest()
            .evm_address(&evm_mapping.account_id().to_string())
            .unwrap()
    );

    assert_eq!(
        evm_mapping.account_id().to_string(),
        jormungandr
            .rest()
            .jor_address(&evm_mapping.evm_address().to_string())
            .unwrap()
    );

    assert_ne!(
        evm_mapping_2.account_id().to_string(),
        jormungandr
            .rest()
            .jor_address(&evm_mapping.evm_address().to_string())
            .unwrap()
    );

    assert_ne!(
        evm_mapping_2.evm_address().to_string(),
        jormungandr
            .rest()
            .evm_address(&evm_mapping.account_id().to_string())
            .unwrap()
    );
}


use assert_fs::TempDir;
use chain_crypto::Ed25519;
use jormungandr_automation::jormungandr::Starter;
use jormungandr_automation::testing::keys::create_new_key_pair;
use jormungandr_automation::testing::time::{get_current_date, wait_for_epoch};
use jormungandr_lib::interfaces::{BlockContentMaxSize, ConfigParam, ConfigParams};
use thor::{FragmentSender, FragmentSenderSetup, FragmentVerifier, TransactionHash};

#[test]
pub fn evm_mapping_jcli_test() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = thor::Wallet::default();
    let bft_secret = create_new_key_pair::<Ed25519>();
    let wallet_initial_funds = 1_000_000;

    let config = ConfigurationBuilder::new()
        .with_funds(vec![alice.to_initial_fund(wallet_initial_funds)])
        .with_consensus_leaders_ids(vec![bft_secret.identifier().into()])
        .with_proposal_expiry_epochs(2)
        .with_slots_per_epoch(10)
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let current_epoch = get_current_date(&mut jormungandr.rest()).epoch();

    let until = BlockDate {
        epoch: current_epoch + 2,
        slot_id: 0,
    };

    let fragment_builder = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    );

    let fs = FragmentSenderSetup::no_verify();

    let fragment_sender = FragmentSender::from_with_setup(jormungandr.block0_configuration(), fs);

    let evm_mapping = TestGen::evm_mapping_for_wallet(&alice.clone().into());

    let alice_fragment = fragment_builder.evm_mapping(&alice, &evm_mapping).encode();

    let jcli = JCli::default();
    jcli.fragment_sender(&jormungandr)
        .send(&alice_fragment)
        .assert_in_block();
    //fragment_sender.send_fragment(&mut alice, alice_fragment.clone(), &jormungandr);

    println!(
        "{:?}",
        jormungandr
            .rest()
            .raw()
            .evm_address(evm_mapping.account_id().to_string())
            .unwrap()
    );

    println!(
        "{:?}",
        jormungandr
            .rest()
            .raw()
            .jor_address(evm_mapping.evm_address().to_string())
            .unwrap()
    );

    wait_for_epoch(current_epoch + 2, jormungandr.rest());

    println!(
        "{:?}",
        jormungandr
            .rest()
            .raw()
            .evm_address(evm_mapping.account_id().to_string())
            .unwrap()
    );

    println!(
        "{:?}",
        jormungandr
            .rest()
            .raw()
            .jor_address(evm_mapping.evm_address().to_string())
            .unwrap()
    );

    wait_for_epoch(current_epoch + 4, jormungandr.rest());

    println!(
        "{:?}",
        jormungandr
            .rest()
            .raw()
            .evm_address(evm_mapping.account_id().to_string())
            .unwrap()
    );

    println!(
        "{:?}",
        jormungandr
            .rest()
            .raw()
            .jor_address(evm_mapping.evm_address().to_string())
            .unwrap()
    );
}
