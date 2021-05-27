use crate::common::jormungandr::{ConfigurationBuilder, Starter};
use crate::common::startup;
use assert_fs::TempDir;
use jormungandr_lib::interfaces::BlockDate;
use jormungandr_lib::interfaces::InitialUTxO;
use jormungandr_lib::interfaces::Mempool;
use jormungandr_testing_utils::testing::node::time;
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
        .config(leader_config.clone())
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let fragment_sender = jormungandr.fragment_sender(FragmentSenderSetup::no_verify());

    let verifier = jormungandr
        .correct_state_verifier()
        .record_wallets_state(vec![&sender, &receiver]);

    let mempool_check = fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .unwrap()
        .assert_size(1)
        .assert_contains_only(mempool_check.fragment_id());

    FragmentVerifier
        .wait_and_verify_is_in_block(Duration::from_secs(2), mempool_check, &jormungandr)
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
        .config(config.clone())
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_wallets_state(vec![&sender, &receiver]);

    let fragment_sender = jormungandr.fragment_sender(FragmentSenderSetup::no_verify());

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .unwrap()
        .assert_empty();

    time::wait_for_date(BlockDate::new(0, 5).into(), jormungandr.explorer());
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
        .with_explorer()
        .with_slot_duration(1)
        .with_mempool(Mempool {
            pool_max_entries: 1.into(),
            log_max_entries: 1.into(),
            persistent_log: None,
        })
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .config(config.clone())
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_wallets_state(vec![&sender, &receiver]);

    let fragment_sender = jormungandr.fragment_sender(FragmentSenderSetup::no_verify());

    let first_fragment = fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    let _second_fragment = fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .unwrap()
        .assert_size(1)
        .assert_contains_only(first_fragment.fragment_id());

    FragmentVerifier
        .wait_and_verify_is_in_block(Duration::from_secs(15), first_fragment, &jormungandr)
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
        .config(config.clone())
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_wallets_state(vec![&sender, &receiver]);

    let fragment_sender = jormungandr.fragment_sender(FragmentSenderSetup::no_verify());

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .unwrap()
        .assert_empty();

    time::wait_for_date(BlockDate::new(0, 5).into(), jormungandr.explorer());

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
        .config(config.clone())
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let verifier = jormungandr
        .correct_state_verifier()
        .record_wallets_state(vec![&sender, &receiver]);

    let fragment_sender = jormungandr.fragment_sender(FragmentSenderSetup::no_verify());

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .unwrap()
        .assert_size(2);

    time::wait_for_date(BlockDate::new(0, 10).into(), jormungandr.explorer());

    verifier
        .value_moved_between_wallets(&sender, &receiver, 2.into())
        .unwrap();
}
