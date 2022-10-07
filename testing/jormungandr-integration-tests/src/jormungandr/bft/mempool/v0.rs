use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use jormungandr_automation::{
    jormungandr::{Block0ConfigurationBuilder, NodeConfigBuilder},
    testing::time,
};
use jormungandr_lib::interfaces::{BlockDate, InitialUTxO, Mempool, SlotDuration};
use std::time::Duration;
use thor::{FragmentSender, FragmentSenderSetup, FragmentVerifier, VerifyExitStrategy};

#[test]
pub fn test_mempool_pool_max_entries_limit() {
    let temp_dir = TempDir::new().unwrap();

    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let leader_block0_config = Block0ConfigurationBuilder::default()
        .with_utxos(vec![
            sender.to_initial_fund(100),
            receiver.to_initial_fund(100),
        ])
        // Use a long slot time to avoid producing a block
        // before both test requests has been sent
        .with_slot_duration(SlotDuration::new(15).unwrap());

    let leader_node_config = NodeConfigBuilder::default().with_mempool(Mempool {
        pool_max_entries: 1.into(),
        log_max_entries: 100.into(),
        persistent_log: None,
    });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(leader_block0_config)
        .with_node_config(leader_node_config)
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::from_settings_with_setup(
        &jormungandr.rest().settings().unwrap(),
        FragmentSenderSetup::no_verify(),
    );

    let verifier = jormungandr
        .correct_state_verifier()
        .record_address_state(vec![&sender.address(), &receiver.address()]);

    let mempool_check = fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    // Wait until the fragment enters the mempool
    FragmentVerifier::wait_fragment(
        Duration::from_millis(100),
        mempool_check.clone(),
        VerifyExitStrategy::OnPending,
        &jormungandr,
    )
    .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_size(1)
        .assert_contains_only(mempool_check.fragment_id());

    FragmentVerifier::wait_and_verify_is_in_block(
        Duration::from_secs(2),
        mempool_check,
        &jormungandr,
    )
    .unwrap();

    verifier
        .value_moved_between_addresses(&sender.address(), &receiver.address(), 1.into())
        .unwrap();
}

#[test]
pub fn test_mempool_pool_max_entries_equal_0() {
    let temp_dir = TempDir::new().unwrap();

    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let config = Block0ConfigurationBuilder::default().with_utxos(vec![
        InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        },
        InitialUTxO {
            address: receiver.address(),
            value: 100.into(),
        },
    ]);

    let node_config = NodeConfigBuilder::default().with_mempool(Mempool {
        pool_max_entries: 0.into(),
        log_max_entries: 100.into(),
        persistent_log: None,
    });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(config)
        .with_node_config(node_config)
        .build()
        .start_node(temp_dir)
        .unwrap();

    let settings = jormungandr.rest().settings().unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_address_state(vec![&sender.address(), &receiver.address()]);

    let fragment_sender =
        FragmentSender::from_settings_with_setup(&settings, FragmentSenderSetup::no_verify());

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_empty();

    time::wait_for_date(BlockDate::new(0, 5), jormungandr.rest());
    verifier
        .no_changes(vec![&sender.address(), &receiver.address()])
        .unwrap();
}

#[test]
pub fn test_mempool_log_max_entries_only_one_fragment() {
    let temp_dir = TempDir::new().unwrap();

    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let config = Block0ConfigurationBuilder::default()
        // Use a long slot time to avoid producing a block
        // before both test requests has been sent
        .with_slot_duration(15.try_into().unwrap())
        .with_utxos(vec![
            InitialUTxO {
                address: sender.address(),
                value: 100.into(),
            },
            InitialUTxO {
                address: receiver.address(),
                value: 100.into(),
            },
        ]);

    let node_config = NodeConfigBuilder::default().with_mempool(Mempool {
        pool_max_entries: 1.into(),
        log_max_entries: 1.into(),
        persistent_log: None,
    });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(config)
        .with_node_config(node_config)
        .build()
        .start_node(temp_dir)
        .unwrap();

    let settings = jormungandr.rest().settings().unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_address_state(vec![&sender.address(), &receiver.address()]);

    let fragment_sender =
        FragmentSender::from_settings_with_setup(&settings, FragmentSenderSetup::no_verify());

    let first_fragment = fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    let _second_fragment = fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    // Wait until the fragment enters the mempool
    FragmentVerifier::wait_fragment(
        Duration::from_millis(100),
        first_fragment.clone(),
        VerifyExitStrategy::OnPending,
        &jormungandr,
    )
    .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_size(1)
        .assert_contains_only(first_fragment.fragment_id());

    FragmentVerifier::wait_and_verify_is_in_block(
        Duration::from_secs(15),
        first_fragment,
        &jormungandr,
    )
    .unwrap();

    verifier
        .value_moved_between_addresses(&sender.address(), &receiver.address(), 1.into())
        .unwrap();
}

#[test]
pub fn test_mempool_log_max_entries_equals_0() {
    let temp_dir = TempDir::new().unwrap();

    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let config = Block0ConfigurationBuilder::default().with_utxos(vec![
        InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        },
        InitialUTxO {
            address: receiver.address(),
            value: 100.into(),
        },
    ]);

    let node_config_builder = NodeConfigBuilder::default().with_mempool(Mempool {
        pool_max_entries: 0.into(),
        log_max_entries: 0.into(),
        persistent_log: None,
    });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(config)
        .with_node_config(node_config_builder)
        .build()
        .start_node(temp_dir)
        .unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_address_state(vec![&sender.address(), &receiver.address()]);

    let settings = jormungandr.rest().settings().unwrap();

    let fragment_sender =
        FragmentSender::from_settings_with_setup(&settings, FragmentSenderSetup::no_verify());

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_empty();

    time::wait_for_date(BlockDate::new(0, 5), jormungandr.rest());

    verifier
        .no_changes(vec![&sender.address(), &receiver.address()])
        .unwrap();
}

#[test]
pub fn test_mempool_pool_max_entries_overrides_log_max_entries() {
    let temp_dir = TempDir::new().unwrap();

    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let config = Block0ConfigurationBuilder::default().with_utxos(vec![
        InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        },
        InitialUTxO {
            address: receiver.address(),
            value: 100.into(),
        },
    ]);

    let node_config_builder = NodeConfigBuilder::default().with_mempool(Mempool {
        pool_max_entries: 2.into(),
        log_max_entries: 0.into(),
        persistent_log: None,
    });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(config)
        .with_node_config(node_config_builder)
        .build()
        .start_node(temp_dir)
        .unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_address_state(vec![&sender.address(), &receiver.address()]);

    let fragment_sender = FragmentSender::try_from(&jormungandr).unwrap();

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    let second_transaction = fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    // Wait until the fragment enters the mempool
    FragmentVerifier::wait_fragment(
        Duration::from_millis(100),
        second_transaction,
        VerifyExitStrategy::OnPending,
        &jormungandr,
    )
    .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_size(2);

    time::wait_for_date(BlockDate::new(0, 10), jormungandr.rest());

    verifier
        .value_moved_between_addresses(&sender.address(), &receiver.address(), 2.into())
        .unwrap();
}
