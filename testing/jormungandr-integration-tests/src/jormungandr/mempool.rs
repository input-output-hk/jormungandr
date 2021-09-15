use crate::common::jormungandr::{starter::Role, Starter};
use crate::common::{jormungandr::ConfigurationBuilder, startup};
use assert_fs::fixture::{PathChild, PathCreateDir};
use assert_fs::TempDir;
use chain_impl_mockchain::fee::LinearFee;
use chain_impl_mockchain::{block::BlockDate, chaintypes::ConsensusVersion};
use jormungandr_lib::interfaces::InitialUTxO;
use jormungandr_lib::interfaces::PersistentLog;
use jormungandr_lib::interfaces::{BlockDate as BlockDateDto, Mempool};
use jormungandr_testing_utils::testing::fragments::FragmentExporter;
use jormungandr_testing_utils::testing::fragments::PersistentLogViewer;
use jormungandr_testing_utils::testing::{
    node::time, FragmentGenerator, FragmentSender, FragmentSenderSetup, FragmentVerifier,
    MemPoolCheck,
};
use jormungandr_testing_utils::testing::{AdversaryFragmentSender, AdversaryFragmentSenderSetup};
use jortestkit::prelude::Wait;
use std::fs::metadata;
use std::path::Path;
use std::thread::sleep;
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
            .with_block_content_max_size(10000)
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
        chain_impl_mockchain::block::BlockDate {
            epoch: 10,
            slot_id: 0,
        }
        .into(),
        FragmentSenderSetup::dump_into(dump_folder.path().to_path_buf()),
    );

    let time_era = jormungandr.time_era();

    let mut fragment_generator = FragmentGenerator::new(
        sender,
        receiver,
        jormungandr.to_remote(),
        time_era.slots_per_epoch(),
        2,
        2,
        2,
        fragment_sender,
    );

    fragment_generator.prepare(BlockDateDto::new(1, 0));

    time::wait_for_epoch(1, jormungandr.rest());

    let wait = Wait::new(Duration::from_secs(1), 25);
    FragmentVerifier::wait_until_all_processed(wait, &jormungandr).unwrap();

    let mem_checks: Vec<MemPoolCheck> = fragment_generator.send_all().unwrap();
    FragmentVerifier::wait_and_verify_all_are_in_block(
        Duration::from_secs(2),
        mem_checks,
        &jormungandr,
    )
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
        BlockDate::first().next_epoch().into(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    );

    adversary_sender
        .send_transactions_with_invalid_counter(10, &mut sender, &receiver, &jormungandr)
        .unwrap();

    sleep(Duration::from_secs(1));

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
    assert!(std::fs::read_dir(&path).unwrap().count() > 0);
}

#[test]
pub fn invalid_folder() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump");
    let persistent_log_path = dump_folder.child("/dev/null/foo::///;log");

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
        BlockDate::first().next_epoch().into(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    );

    adversary_sender
        .send_transactions_with_invalid_counter(10, &mut sender, &receiver, &jormungandr)
        .unwrap();

    sleep(Duration::from_secs(1));

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
        BlockDate::first().next_epoch().into(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    );

    adversary_sender
        .send_transactions_with_invalid_counter(10, &mut sender, &receiver, &jormungandr)
        .unwrap();

    sleep(Duration::from_secs(1));

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
        BlockDate::first().next_epoch().into(),
        FragmentSenderSetup::dump_into(dump_folder.path().to_path_buf()),
    );

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    sleep(Duration::from_secs(1));

    let persistent_log_viewer = PersistentLogViewer::new(persistent_log_path.path().to_path_buf());

    assert_eq!(1, persistent_log_viewer.get_all().len());

    let fragment_logs = jormungandr.rest().fragment_logs().unwrap();

    assert_eq!(fragment_logs.len(), 1);
    assert!(fragment_logs.values().next().unwrap().is_pending());
}

#[test]
pub fn node_should_pickup_log_after_restart() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump_folder");
    let persistent_log_path = temp_dir.child("persistent_log");
    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let config = ConfigurationBuilder::new()
        .with_slots_per_epoch(60)
        .with_slot_duration(3)
        .with_explorer()
        .with_mempool(Mempool {
            pool_max_entries: 1usize.into(),
            log_max_entries: 1000usize.into(),
            persistent_log: Some(PersistentLog {
                dir: persistent_log_path.path().to_path_buf(),
            }),
        })
        .with_block0_consensus(ConsensusVersion::Bft)
        .with_funds(vec![
            InitialUTxO {
                address: sender.address(),
                value: 1_000_000.into(),
            },
            InitialUTxO {
                address: receiver.address(),
                value: 1_000_000.into(),
            },
        ])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .config(config.clone())
        .role(Role::Leader)
        .start()
        .unwrap();

    let adversary_sender = AdversaryFragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDate::first().next_epoch().into(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    );

    adversary_sender
        .send_transactions_with_invalid_counter(10, &mut sender, &receiver, &jormungandr)
        .unwrap();

    sleep(Duration::from_secs(1));

    jormungandr.stop();

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .role(Role::Leader)
        .start()
        .unwrap();

    let adversary_sender = AdversaryFragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDate::first().next_epoch().into(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    );

    adversary_sender
        .send_transactions_with_invalid_counter(10, &mut sender, &receiver, &jormungandr)
        .unwrap();

    sleep(Duration::from_secs(1));

    let persistent_log_viewer = PersistentLogViewer::new(persistent_log_path.path().to_path_buf());

    assert_eq!(20, persistent_log_viewer.get_all().len());
}

#[test]
/// Verifies that a leader node will reject a fragment that has expired, even after it's been
/// accepted in its mempool.
pub fn expired_fragment_should_be_rejected_by_leader_praos_node() {
    const N_FRAGMENTS: u32 = 10;

    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[receiver.clone()],
        ConfigurationBuilder::new()
            .with_block_content_max_size(256) // This should only fit 1 transaction
            .with_slots_per_epoch(N_FRAGMENTS)
            .with_slot_duration(1)
            .with_mempool(Mempool {
                pool_max_entries: 1000.into(),
                log_max_entries: 1000.into(),
                persistent_log: None,
            })
            .with_log_level("debug".into()),
    )
    .unwrap();

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        LinearFee::new(0, 0, 0),
        BlockDate::first().next_epoch().into(),
        FragmentSenderSetup::no_verify(),
    );

    for i in 0..N_FRAGMENTS as u64 {
        fragment_sender
            .send_transaction(&mut sender, &receiver, &jormungandr, (100 + i).into())
            .unwrap();
    }

    let check = fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

    // By the time the rest of the transactions have been placed in blocks, the epoch should be over
    // and the transaction below should have expired.
    FragmentVerifier::wait_and_verify_is_rejected(Duration::from_secs(1), check, &jormungandr)
        .unwrap();
}

#[test]
/// Verifies that a passive node will reject a fragment that has expired, even after it's been
/// accepted in its mempool.
fn expired_fragment_should_be_rejected_by_passive_bft_node() {
    const N_FRAGMENTS: u32 = 10;

    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let leader = startup::start_bft(
        vec![&receiver, &sender],
        ConfigurationBuilder::new()
            .with_block_content_max_size(256) // This should only fit 1 transaction
            .with_slots_per_epoch(N_FRAGMENTS)
            .with_slot_duration(1)
            .with_mempool(Mempool {
                pool_max_entries: 1000.into(),
                log_max_entries: 1000.into(),
                persistent_log: None,
            })
            .with_log_level("debug".into()),
    )
    .unwrap();

    let passive_dir = TempDir::new().unwrap().child("passive");
    passive_dir.create_dir_all().unwrap();

    let passive = Starter::new()
        .config(
            ConfigurationBuilder::new()
                .with_trusted_peers(vec![leader.to_trusted_peer()])
                .with_block_hash(&leader.genesis_block_hash().to_string())
                .with_log_level("debug".into())
                .build(&passive_dir),
        )
        .passive()
        .start()
        .unwrap();

    leader
        .wait_for_bootstrap(&StartupVerificationMode::Rest, Duration::from_secs(30))
        .unwrap();
    println!("{:?}", std::time::SystemTime::now());

    passive
        .wait_for_bootstrap(&StartupVerificationMode::Rest, Duration::from_secs(30))
        .unwrap();
    println!("{:?}", std::time::SystemTime::now());

    let fragment_sender = FragmentSender::new(
        passive.genesis_block_hash(),
        LinearFee::new(0, 0, 0),
        BlockDate::first().next_epoch().into(),
        FragmentSenderSetup::no_verify(),
    );

    for i in 0..N_FRAGMENTS as u64 {
        fragment_sender
            .send_transaction(&mut sender, &receiver, &passive, (100 + i).into())
            .unwrap();
    }

    let check = fragment_sender
        .send_transaction(&mut sender, &receiver, &passive, 1.into())
        .unwrap();

    // By the time the rest of the transactions have been placed in blocks, the epoch should be over
    // and the transaction below should have expired.
    FragmentVerifier::wait_and_verify_is_rejected(Duration::from_secs(1), check, &passive).unwrap();
}
