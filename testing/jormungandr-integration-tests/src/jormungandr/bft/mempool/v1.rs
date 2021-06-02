use crate::common::jormungandr::{ConfigurationBuilder, Starter};
use crate::common::startup;
use assert_fs::TempDir;
use chain_core::property::Fragment;
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_lib::interfaces::BlockDate;
use jormungandr_lib::interfaces::FragmentRejectionReason;
use jormungandr_lib::interfaces::FragmentsProcessingSummary;
use jormungandr_lib::interfaces::InitialUTxO;
use jormungandr_lib::interfaces::Mempool;
use jormungandr_testing_utils::testing::node::time;
use jormungandr_testing_utils::testing::node::RestError;
use jormungandr_testing_utils::testing::MemPoolCheck;
use jormungandr_testing_utils::testing::{FragmentSenderSetup, FragmentVerifier};
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
        .with_explorer()
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
            receiver.address(),
            1.into(),
        )
        .unwrap();

    sender.confirm_transaction();

    let second_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    let mempools = assert_accepted_rejected(
        vec![first_transaction.id()],
        vec![
            (
                second_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 0 },
            ),
            (
                second_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 1 },
            ),
        ],
        jormungandr
            .rest()
            .send_fragment_batch(vec![first_transaction, second_transaction], false),
    );

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .unwrap()
        .assert_size(1)
        .assert_contains_only(mempools[0].fragment_id());

    FragmentVerifier
        .wait_and_verify_is_in_block(Duration::from_secs(2), mempools[0].clone(), &jormungandr)
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
        .with_explorer()
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
            receiver.address(),
            1.into(),
        )
        .unwrap();

    sender.confirm_transaction();

    let second_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    assert_accepted_rejected(
        vec![],
        vec![
            (
                first_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 0 },
            ),
            (
                second_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 0 },
            ),
            (
                first_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 1 },
            ),
            (
                second_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 1 },
            ),
        ],
        jormungandr
            .rest()
            .send_fragment_batch(vec![first_transaction, second_transaction], false),
    );

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .unwrap()
        .assert_empty();

    time::wait_for_date(BlockDate::new(0, 10), jormungandr.explorer());
    verifier.no_changes(vec![&sender, &receiver]).unwrap();
}

pub fn assert_accepted_rejected(
    accepted: Vec<FragmentId>,
    rejected: Vec<(FragmentId, FragmentRejectionReason)>,
    result: Result<Vec<MemPoolCheck>, RestError>,
) -> Vec<MemPoolCheck> {
    match result.err().unwrap() {
        RestError::NonSuccessErrorCode {
            checks,
            status,
            response,
        } => {
            let summary: FragmentsProcessingSummary = serde_json::from_str(&response).unwrap();
            if rejected.len() > 0 {
                assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
            }
            assert_eq!(summary.accepted, accepted);
            assert_eq!(
                summary
                    .rejected
                    .iter()
                    .map(|x| (x.id.clone(), x.reason.clone()))
                    .collect::<Vec<(FragmentId, FragmentRejectionReason)>>(),
                rejected
            );

            checks
        }
        _ => panic!("wrong error code"),
    }
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
        .with_explorer()
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
            receiver.address(),
            1.into(),
        )
        .unwrap();

    sender.confirm_transaction();

    let second_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    let mempools = assert_accepted_rejected(
        vec![first_transaction.id()],
        vec![
            (
                second_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 0 },
            ),
            (
                second_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 1 },
            ),
        ],
        jormungandr
            .rest()
            .send_fragment_batch(vec![first_transaction, second_transaction], false),
    );

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .unwrap()
        .assert_size(1)
        .assert_contains_only(mempools[0].fragment_id());

    FragmentVerifier
        .wait_and_verify_is_in_block(Duration::from_secs(12), mempools[0].clone(), &jormungandr)
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
        .with_explorer()
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
            receiver.address(),
            1.into(),
        )
        .unwrap();

    sender.confirm_transaction();

    let second_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    assert_accepted_rejected(
        vec![],
        vec![
            (
                first_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 0 },
            ),
            (
                second_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 0 },
            ),
            (
                first_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 1 },
            ),
            (
                second_transaction.id(),
                FragmentRejectionReason::PoolOverflow { pool_number: 1 },
            ),
        ],
        jormungandr
            .rest()
            .send_fragment_batch(vec![first_transaction, second_transaction], false),
    );

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .unwrap()
        .assert_empty();

    time::wait_for_date(BlockDate::new(0, 10), jormungandr.explorer());

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
        .with_explorer()
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
            receiver.address(),
            1.into(),
        )
        .unwrap();

    sender.confirm_transaction();

    let second_transaction = sender
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            receiver.address(),
            1.into(),
        )
        .unwrap();

    fragment_sender
        .send_batch_fragments(
            vec![first_transaction, second_transaction],
            false,
            &jormungandr,
        )
        .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .unwrap()
        .assert_size(2);

    time::wait_for_date(BlockDate::new(0, 10), jormungandr.explorer());

    verifier
        .value_moved_between_wallets(&sender, &receiver, 2.into())
        .unwrap();
}
