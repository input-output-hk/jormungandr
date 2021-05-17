use crate::common::jormungandr::{starter::Role, JormungandrProcess, Starter};
use crate::common::{jormungandr::ConfigurationBuilder, startup};
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use chain_impl_mockchain::chaintypes::ConsensusVersion;
use jormungandr_lib::interfaces::InitialUTxO;
use jormungandr_lib::interfaces::PersistentLog;
use jormungandr_lib::interfaces::{BlockDate, Mempool};
use jormungandr_testing_utils::testing::fragments::FragmentExporter;
use jormungandr_testing_utils::testing::fragments::PersistentLogViewer;
use jormungandr_testing_utils::testing::{
    node::time, FaultyTransactionBuilder, FragmentGenerator, FragmentSender, FragmentSenderSetup,
    FragmentVerifier, MemPoolCheck,
};
use jormungandr_testing_utils::testing::{AdversaryFragmentSender, AdversaryFragmentSenderSetup};
use jormungandr_testing_utils::wallet::Wallet;
use jortestkit::prelude::Wait;
use std::fs::metadata;
use std::path::Path;
use std::time::Duration;

#[test]
pub fn dump_send_correct_fragments() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump");
    let persistent_log_path = temp_dir.child("persistent_log");
    let receiver = startup::create_new_account_address();
    let sender = startup::create_new_account_address();

    let jormungandr = startup::start_bft(
        vec![&sender, &receiver],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_slot_duration(1)
            .with_explorer()
            .with_mempool(Mempool {
                pool_max_entries: 1_000_000usize.into(),
                log_max_entries: 1_000_000usize.into(),
                persistent_log: Some(PersistentLog {
                    dir: persistent_log_path.path().to_path_buf(),
                }),
            }),
    )
    .unwrap();

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::dump_into(dump_folder.path().to_path_buf()),
    );

    let time_era = jormungandr.time_era();

    let mut fragment_generator = FragmentGenerator::new(
        sender.clone(),
        receiver.clone(),
        jormungandr.to_remote(),
        jormungandr.explorer(),
        time_era.slots_per_epoch(),
        2,
        2,
        fragment_sender,
    );

    fragment_generator.prepare(BlockDate::new(1, 0));
    let verifier = FragmentVerifier;

    time::wait_for_epoch(1, jormungandr.explorer());

    let wait = Wait::new(Duration::from_secs(1), 25);
    verifier
        .wait_until_all_processed(wait, &jormungandr)
        .unwrap();

    let mem_checks: Vec<MemPoolCheck> = fragment_generator.send_all().unwrap();
    verifier
        .wait_and_verify_all_are_in_block(Duration::from_secs(2), mem_checks.clone(), &jormungandr)
        .unwrap();

    assert_all_fragment_are_persisted(dump_folder.path(), persistent_log_path.path());
}

#[test]
pub fn dump_send_invalid_fragments() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump");
    let persistent_log_path = temp_dir.child("persistent_log");
    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let jormungandr = startup::start_bft(
        vec![&sender, &receiver],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_slot_duration(1)
            .with_explorer()
            .with_mempool(Mempool {
                pool_max_entries: 1_000_000usize.into(),
                log_max_entries: 1_000_000usize.into(),
                persistent_log: Some(PersistentLog {
                    dir: persistent_log_path.path().to_path_buf(),
                }),
            }),
    )
    .unwrap();

    let adversary_sender = AdversaryFragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    );

    adversary_sender
        .send_transactions_with_invalid_counter(10, &mut sender, &receiver, &jormungandr)
        .unwrap();

    assert_all_fragment_are_persisted(dump_folder.path(), persistent_log_path.path());
}

fn assert_all_fragment_are_persisted<P: AsRef<Path>, R: AsRef<Path>>(left: P, right: R) {
    let exporter = FragmentExporter::new(left.as_ref().to_path_buf()).unwrap();
    let fragments = exporter.read_as_bytes().unwrap();

    let persistent_log_viewer = PersistentLogViewer::new(right.as_ref().to_path_buf());
    assert_eq!(fragments.len(), persistent_log_viewer.get_all().len());
    assert_eq!(fragments, persistent_log_viewer.get_bin());
}

#[test]
pub fn non_existing_folder() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump");
    let persistent_log_path = dump_folder.child("persistent_log");
    let receiver = startup::create_new_account_address();
    let sender = startup::create_new_account_address();

    let _jormungandr = startup::start_bft(
        vec![&sender, &receiver],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_slot_duration(1)
            .with_explorer()
            .with_mempool(Mempool {
                pool_max_entries: 1_000_000usize.into(),
                log_max_entries: 1_000_000usize.into(),
                persistent_log: Some(PersistentLog {
                    dir: persistent_log_path.path().to_path_buf(),
                }),
            }),
    )
    .unwrap();

    let path = persistent_log_path.path();

    assert!(path.exists());
    assert!(metadata(path).unwrap().is_dir());
    assert_eq!(std::fs::read_dir(&path).unwrap().count(), 1);
}

#[test]
pub fn invalid_folder() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump");
    let persistent_log_path = dump_folder.child("persist::///;ent_log");

    let config = ConfigurationBuilder::new()
        .with_mempool(Mempool {
            pool_max_entries: 1_000_000usize.into(),
            log_max_entries: 1_000_000usize.into(),
            persistent_log: Some(PersistentLog {
                dir: persistent_log_path.path().to_path_buf(),
            }),
        })
        .build(&temp_dir);

    Starter::new()
        .config(config)
        .start_fail("failed to open persistent log file");
}

#[test]
pub fn fragment_which_reached_mempool_should_be_persisted() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump_folder");
    let persistent_log_path = temp_dir.child("persistent_log");
    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let jormungandr = startup::start_bft(
        vec![&sender, &receiver],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_slot_duration(3)
            .with_explorer()
            .with_mempool(Mempool {
                pool_max_entries: 1usize.into(),
                log_max_entries: 1000usize.into(),
                persistent_log: Some(PersistentLog {
                    dir: persistent_log_path.path().to_path_buf(),
                }),
            }),
    )
    .unwrap();

    let adversary_sender = AdversaryFragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    );

    adversary_sender
        .send_transactions_with_invalid_counter(10, &mut sender, &receiver, &jormungandr)
        .unwrap();

    assert_all_fragment_are_persisted(dump_folder.path(), persistent_log_path.path());
}

#[test]
pub fn fragment_which_is_not_in_fragment_log_should_be_persisted() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump_folder");
    let persistent_log_path = temp_dir.child("persistent_log");
    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let jormungandr = startup::start_bft(
        vec![&sender, &receiver],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_slot_duration(3)
            .with_explorer()
            .with_mempool(Mempool {
                pool_max_entries: 1000usize.into(),
                log_max_entries: 1usize.into(),
                persistent_log: Some(PersistentLog {
                    dir: persistent_log_path.path().to_path_buf(),
                }),
            }),
    )
    .unwrap();

    let adversary_sender = AdversaryFragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    );

    adversary_sender
        .send_transactions_with_invalid_counter(10, &mut sender, &receiver, &jormungandr)
        .unwrap();

    assert_all_fragment_are_persisted(dump_folder.path(), persistent_log_path.path());
}

#[test]
pub fn pending_fragment_should_be_persisted() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump_folder");
    let persistent_log_path = temp_dir.child("persistent_log");
    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let jormungandr = startup::start_bft(
        vec![&sender, &receiver],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(5)
            .with_slot_duration(60)
            .with_explorer()
            .with_mempool(Mempool {
                pool_max_entries: 10usize.into(),
                log_max_entries: 10usize.into(),
                persistent_log: Some(PersistentLog {
                    dir: persistent_log_path.path().to_path_buf(),
                }),
            }),
    )
    .unwrap();

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::dump_into(dump_folder.path().to_path_buf()),
    );

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    let persistent_log_viewer = PersistentLogViewer::new(persistent_log_path.path().to_path_buf());

    assert_eq!(1, persistent_log_viewer.get_all().len());

    let fragment_logs = jormungandr.rest().fragment_logs().unwrap();

    assert_eq!(fragment_logs.len(), 1);
    assert!(fragment_logs.values().next().unwrap().is_pending());
}

#[test]
pub fn fragment_with_wrong_block0_hash_should_be_persisted() {
    let temp_dir = TempDir::new().unwrap();
    let persistent_log_path = temp_dir.child("persistent_log");

    let (jormungandr, from, to) = setup(persistent_log_path.path(), &temp_dir);

    let faulty_tx_builder =
        FaultyTransactionBuilder::new(jormungandr.genesis_block_hash(), jormungandr.fees());
    let fragment = faulty_tx_builder.wrong_block0_hash(&from, &to);
    jormungandr.rest().send_fragment(fragment).unwrap();

    assert!(fragment_should_be_persisted(persistent_log_path.path()))
}

#[test]
pub fn fragment_with_no_input_should_not_be_persisted() {
    let temp_dir = TempDir::new().unwrap();
    let persistent_log_path = temp_dir.child("persistent_log");

    let (jormungandr, from, _) = setup(persistent_log_path.path(), &temp_dir);

    let faulty_tx_builder =
        FaultyTransactionBuilder::new(jormungandr.genesis_block_hash(), jormungandr.fees());
    let fragment = faulty_tx_builder.no_input(&from);
    jormungandr.rest().send_fragment(fragment).unwrap();

    assert!(fragment_should_not_be_persisted(persistent_log_path.path()))
}

#[test]
pub fn fragment_with_no_output_should_be_persisted() {
    let temp_dir = TempDir::new().unwrap();
    let persistent_log_path = temp_dir.child("persistent_log");

    let (jormungandr, from, _) = setup(persistent_log_path.path(), &temp_dir);

    let faulty_tx_builder =
        FaultyTransactionBuilder::new(jormungandr.genesis_block_hash(), jormungandr.fees());
    let fragment = faulty_tx_builder.no_output(&from);
    jormungandr.rest().send_fragment(fragment).unwrap();

    assert!(fragment_should_be_persisted(persistent_log_path.path()))
}

#[test]
pub fn unbalanced_fragment_should_not_be_persisted() {
    let temp_dir = TempDir::new().unwrap();
    let persistent_log_path = temp_dir.child("persistent_log");

    let (jormungandr, from, to) = setup(persistent_log_path.path(), &temp_dir);

    let faulty_tx_builder =
        FaultyTransactionBuilder::new(jormungandr.genesis_block_hash(), jormungandr.fees());
    let fragment = faulty_tx_builder.unbalanced(&from, &to);
    jormungandr.rest().send_fragment(fragment).unwrap();

    assert!(fragment_should_not_be_persisted(persistent_log_path.path()))
}

#[test]
pub fn empty_fragment_should_be_persisted() {
    let temp_dir = TempDir::new().unwrap();
    let persistent_log_path = temp_dir.child("persistent_log");

    let (jormungandr, _, _) = setup(persistent_log_path.path(), &temp_dir);

    let faulty_tx_builder =
        FaultyTransactionBuilder::new(jormungandr.genesis_block_hash(), jormungandr.fees());
    let fragment = faulty_tx_builder.empty();
    jormungandr.rest().send_fragment(fragment).unwrap();

    assert!(fragment_should_be_persisted(persistent_log_path.path()))
}

#[test]
pub fn fragment_with_no_witnesses_should_not_be_persisted() {
    let temp_dir = TempDir::new().unwrap();
    let persistent_log_path = temp_dir.child("persistent_log");

    let (jormungandr, from, to) = setup(persistent_log_path.path(), &temp_dir);

    let faulty_tx_builder =
        FaultyTransactionBuilder::new(jormungandr.genesis_block_hash(), jormungandr.fees());
    let fragment = faulty_tx_builder.no_witnesses(&from, &to);
    jormungandr.rest().send_fragment(fragment).unwrap();

    assert!(fragment_should_not_be_persisted(persistent_log_path.path()))
}

fn fragment_should_be_persisted<P: AsRef<Path>>(persistent_log_path: P) -> bool {
    std::thread::sleep(std::time::Duration::from_secs(5));
    let persistent_log_viewer =
        PersistentLogViewer::new(persistent_log_path.as_ref().to_path_buf());
    1 == persistent_log_viewer.get_all().len()
}

fn fragment_should_not_be_persisted<P: AsRef<Path>>(persistent_log_path: P) -> bool {
    !fragment_should_be_persisted(persistent_log_path)
}

fn setup<P: AsRef<Path>>(
    persistent_log_path: P,
    temp_dir: &TempDir,
) -> (JormungandrProcess, Wallet, Wallet) {
    let to = startup::create_new_account_address();
    let from = startup::create_new_account_address();

    let config = ConfigurationBuilder::new()
        .with_slots_per_epoch(60)
        .with_slot_duration(3)
        .with_explorer()
        .with_mempool(Mempool {
            pool_max_entries: 1usize.into(),
            log_max_entries: 1000usize.into(),
            persistent_log: Some(PersistentLog {
                dir: persistent_log_path.as_ref().to_path_buf(),
            }),
        })
        .with_block0_consensus(ConsensusVersion::Bft)
        .with_funds(vec![
            InitialUTxO {
                address: from.address(),
                value: 1_000_000.into(),
            },
            InitialUTxO {
                address: to.address(),
                value: 1_000_000.into(),
            },
        ])
        .build(temp_dir);

    let jormungandr = Starter::new()
        .config(config.clone())
        .role(Role::Leader)
        .start()
        .unwrap();

    (jormungandr, from, to)
}
