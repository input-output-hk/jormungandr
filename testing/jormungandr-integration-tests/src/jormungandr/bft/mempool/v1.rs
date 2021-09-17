use jormungandr_testing_utils::testing::common::jormungandr::{ConfigurationBuilder, Starter};
use jormungandr_testing_utils::testing::common::startup;
use assert_fs::TempDir;
use chain_core::property::Fragment;
use chain_impl_mockchain::block::BlockDate;
use jormungandr_lib::interfaces::BlockDate as BlockDateDto;
use jormungandr_lib::interfaces::FragmentRejectionReason;
use jormungandr_lib::interfaces::InitialUTxO;
use jormungandr_lib::interfaces::Mempool;
use jormungandr_testing_utils::testing::node::assert_accepted_rejected;
use jormungandr_testing_utils::testing::node::time;
use jormungandr_testing_utils::testing::{
    fragments::VerifyExitStrategy, FragmentSenderSetup, FragmentVerifier,
};
use std::time::Duration;

#[test]
pub fn test_mempool_pool_max_entries_limit() {
    let temp_dir = TempDir::new().unwrap();

    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let leader_config = ConfigurationBuilder::new()
        .with_funds(vec![
            InitialUTxO {
                address: sender.address(),
                value: 100.into(),
            },
            InitialUTxO {
                address: receiver.address(),
                value: 100.into(),
            },
        ])
        .with_slot_duration(2)
        .with_mempool(Mempool {
            pool_max_entries: 1.into(),
            log_max_entries: 100.into(),
            persistent_log: None,
        })
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .config(leader_config)
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_wallets_state(vec![&sender, &receiver]);

    let first_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    sender.confirm_transaction();

    let second_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    let mempools = assert_accepted_rejected(
        vec![first_transaction.id()],
        vec![(
            second_transaction.id(),
            FragmentRejectionReason::PoolOverflow,
        )],
        jormungandr
            .rest()
            .send_fragment_batch(vec![first_transaction, second_transaction], false),
    );

    // Wait until the fragment enters the mempool
    FragmentVerifier::wait_fragment(
        Duration::from_millis(100),
        mempools[0].clone(),
        VerifyExitStrategy::OnPending,
        &jormungandr,
    )
    .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_size(1)
        .assert_contains_only(mempools[0].fragment_id());

    FragmentVerifier::wait_and_verify_is_in_block(
        Duration::from_secs(2),
        mempools[0].clone(),
        &jormungandr,
    )
    .unwrap();

    verifier
        .value_moved_between_wallets(&sender, &receiver, 1.into())
        .unwrap();
}

#[test]
pub fn test_mempool_pool_max_entries_equal_0() {
    let temp_dir = TempDir::new().unwrap();

    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            InitialUTxO {
                address: sender.address(),
                value: 100.into(),
            },
            InitialUTxO {
                address: receiver.address(),
                value: 100.into(),
            },
        ])
        .with_slot_duration(1)
        .with_mempool(Mempool {
            pool_max_entries: 0.into(),
            log_max_entries: 100.into(),
            persistent_log: None,
        })
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .config(config)
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_wallets_state(vec![&sender, &receiver]);

    let first_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    sender.confirm_transaction();

    let second_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    assert_accepted_rejected(
        vec![],
        vec![
            (
                first_transaction.id(),
                FragmentRejectionReason::PoolOverflow,
            ),
            (
                second_transaction.id(),
                FragmentRejectionReason::PoolOverflow,
            ),
        ],
        jormungandr
            .rest()
            .send_fragment_batch(vec![first_transaction, second_transaction], false),
    );

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_empty();

    time::wait_for_date(BlockDateDto::new(0, 10), jormungandr.rest());
    verifier.no_changes(vec![&sender, &receiver]).unwrap();
}

#[test]
pub fn test_mempool_log_max_entries_only_one_fragment() {
    let temp_dir = TempDir::new().unwrap();

    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            InitialUTxO {
                address: sender.address(),
                value: 100.into(),
            },
            InitialUTxO {
                address: receiver.address(),
                value: 100.into(),
            },
        ])
        .with_slot_duration(1)
        .with_mempool(Mempool {
            pool_max_entries: 1.into(),
            log_max_entries: 1.into(),
            persistent_log: None,
        })
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .config(config)
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_wallets_state(vec![&sender, &receiver]);

    let first_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    sender.confirm_transaction();

    let second_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    let mempools = assert_accepted_rejected(
        vec![first_transaction.id()],
        vec![(
            second_transaction.id(),
            FragmentRejectionReason::PoolOverflow,
        )],
        jormungandr
            .rest()
            .send_fragment_batch(vec![first_transaction, second_transaction], false),
    );

    // Wait until the fragment enters the mempool
    FragmentVerifier::wait_fragment(
        Duration::from_millis(100),
        mempools[0].clone(),
        VerifyExitStrategy::OnPending,
        &jormungandr,
    )
    .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_size(1)
        .assert_contains_only(mempools[0].fragment_id());

    FragmentVerifier::wait_and_verify_is_in_block(
        Duration::from_secs(12),
        mempools[0].clone(),
        &jormungandr,
    )
    .unwrap();

    verifier
        .value_moved_between_wallets(&sender, &receiver, 1.into())
        .unwrap();
}

#[test]
pub fn test_mempool_log_max_entries_equals_0() {
    let temp_dir = TempDir::new().unwrap();

    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            InitialUTxO {
                address: sender.address(),
                value: 100.into(),
            },
            InitialUTxO {
                address: receiver.address(),
                value: 100.into(),
            },
        ])
        .with_slot_duration(1)
        .with_mempool(Mempool {
            pool_max_entries: 0.into(),
            log_max_entries: 0.into(),
            persistent_log: None,
        })
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .config(config)
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_wallets_state(vec![&sender, &receiver]);

    let first_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    sender.confirm_transaction();

    let second_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    assert_accepted_rejected(
        vec![],
        vec![
            (
                first_transaction.id(),
                FragmentRejectionReason::PoolOverflow,
            ),
            (
                second_transaction.id(),
                FragmentRejectionReason::PoolOverflow,
            ),
        ],
        jormungandr
            .rest()
            .send_fragment_batch(vec![first_transaction, second_transaction], false),
    );

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_empty();

    time::wait_for_date(BlockDateDto::new(0, 10), jormungandr.rest());

    verifier.no_changes(vec![&sender, &receiver]).unwrap();
}

#[test]
pub fn test_mempool_pool_max_entries_overrides_log_max_entries() {
    let temp_dir = TempDir::new().unwrap();

    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            InitialUTxO {
                address: sender.address(),
                value: 100.into(),
            },
            InitialUTxO {
                address: receiver.address(),
                value: 100.into(),
            },
        ])
        .with_slot_duration(1)
        .with_mempool(Mempool {
            pool_max_entries: 2.into(),
            log_max_entries: 0.into(),
            persistent_log: None,
        })
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .config(config)
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_wallets_state(vec![&sender, &receiver]);

    let fragment_sender = jormungandr.fragment_sender(FragmentSenderSetup::no_verify());

    let first_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    sender.confirm_transaction();

    let second_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    let summary = fragment_sender
        .send_batch_fragments(
            vec![first_transaction, second_transaction],
            false,
            &jormungandr,
        )
        .unwrap();

    // Wait until the fragment enters the mempool
    FragmentVerifier::wait_fragment(
        Duration::from_millis(100),
        summary.fragment_ids()[0].into(),
        VerifyExitStrategy::OnPending,
        &jormungandr,
    )
    .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_size(2);

    time::wait_for_date(BlockDateDto::new(0, 10), jormungandr.rest());

    verifier
        .value_moved_between_wallets(&sender, &receiver, 2.into())
        .unwrap();
}
