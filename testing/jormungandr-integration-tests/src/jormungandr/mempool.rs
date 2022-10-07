use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::{fixture::PathChild, TempDir};
use chain_core::property::{FromStr, Serialize};
use chain_crypto::Ed25519;
use chain_impl_mockchain::{
    block::BlockDate,
    chaintypes::ConsensusVersion,
    fee::LinearFee,
    tokens::{identifier::TokenIdentifier, minting_policy::MintingPolicy},
};
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{BlockchainConfiguration, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::{
    jormungandr::{
        Block0ConfigurationBuilder, FragmentNode, JormungandrBootstrapper, MemPoolCheck,
        NodeConfigBuilder,
    },
    testing::{block0::Block0ConfigurationExtension, keys::create_new_key_pair, time},
};
use jormungandr_lib::interfaces::{
    BlockDate as BlockDateDto, InitialToken, InitialUTxO, Mempool, PersistentLog, SlotDuration,
};
use loki::{AdversaryFragmentSender, AdversaryFragmentSenderSetup};
use mjolnir::generators::FragmentGenerator;
use std::{fs::metadata, path::Path, thread::sleep, time::Duration};
use thor::{
    Block0ConfigurationBuilderExtension, BlockDateGenerator, FragmentBuilder, FragmentExporter,
    FragmentSender, FragmentSenderSetup, FragmentVerifier, PersistentLogViewer,
};

#[test]
pub fn dump_send_correct_fragments() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump");
    let persistent_log_path = temp_dir.child("persistent_log");
    let receiver = thor::Wallet::default();
    let sender = thor::Wallet::default();
    let first_bft_leader = create_new_key_pair::<Ed25519>();

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallets_having_some_values(vec![&sender, &receiver])
                .with_slots_per_epoch(60.try_into().unwrap())
                .with_block_content_max_size(100000.into())
                .with_slot_duration(1.try_into().unwrap())
                .with_token(InitialToken {
                    token_id: TokenIdentifier::from_str(
                        "00000000000000000000000000000000000000000000000000000000.00000000",
                    )
                    .unwrap()
                    .into(),
                    policy: MintingPolicy::new().into(),
                    to: vec![sender.to_initial_token(1_000_000)],
                }),
        )
        .with_node_config(NodeConfigBuilder::default().with_mempool(Mempool {
            pool_max_entries: 1_000_000usize.into(),
            log_max_entries: 1_000_000usize.into(),
            persistent_log: Some(PersistentLog {
                dir: persistent_log_path.path().to_path_buf(),
            }),
        }))
        .build()
        .start_node(temp_dir)
        .unwrap();

    let settings = jormungandr.rest().settings().unwrap();

    let fragment_sender = FragmentSender::from_settings_with_setup(
        &settings,
        FragmentSenderSetup::dump_into(dump_folder.path().to_path_buf()),
    );

    let time_era = jormungandr.time_era();

    let mut fragment_generator = FragmentGenerator::new(
        sender,
        receiver,
        Some(first_bft_leader),
        jormungandr.to_remote(),
        time_era.slots_per_epoch(),
        2,
        2,
        2,
        2,
        fragment_sender,
    );

    fragment_generator.prepare(BlockDateDto::new(1, 0));

    time::wait_for_epoch(2, jormungandr.rest());

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
    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallets_having_some_values(vec![&sender, &receiver])
                .with_slots_per_epoch(60.try_into().unwrap()),
        )
        .with_node_config(NodeConfigBuilder::default().with_mempool(Mempool {
            pool_max_entries: 1_000_000usize.into(),
            log_max_entries: 1_000_000usize.into(),
            persistent_log: Some(PersistentLog {
                dir: persistent_log_path.path().to_path_buf(),
            }),
        }))
        .build()
        .start_node(temp_dir)
        .unwrap();

    let adversary_sender = AdversaryFragmentSender::try_from_jormungandr(
        &jormungandr,
        BlockDate::first().next_epoch(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    )
    .unwrap();

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
    let receiver = thor::Wallet::default();
    let sender = thor::Wallet::default();

    let _jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallets_having_some_values(vec![&sender, &receiver])
                .with_slots_per_epoch(60.try_into().unwrap())
                .with_slot_duration(1.try_into().unwrap()),
        )
        .with_node_config(NodeConfigBuilder::default().with_mempool(Mempool {
            pool_max_entries: 1_000_000usize.into(),
            log_max_entries: 1_000_000usize.into(),
            persistent_log: Some(PersistentLog {
                dir: persistent_log_path.path().to_path_buf(),
            }),
        }))
        .build()
        .start_node(temp_dir)
        .unwrap();

    sleep(Duration::from_secs(5)); // give node some time to create the file

    let path = persistent_log_path.path();

    assert!(path.exists());
    assert!(metadata(path).unwrap().is_dir());
    assert!(std::fs::read_dir(path).unwrap().count() > 0);
}

#[test]
pub fn invalid_folder() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump");
    let persistent_log_path = dump_folder.child("/dev/null/foo::///;log");

    let config = NodeConfigBuilder::default().with_mempool(Mempool {
        pool_max_entries: 1_000_000usize.into(),
        log_max_entries: 1_000_000usize.into(),
        persistent_log: Some(PersistentLog {
            dir: persistent_log_path.path().to_path_buf(),
        }),
    });

    SingleNodeTestBootstrapper::default()
        .with_node_config(config)
        .as_bft_leader()
        .build()
        .starter(temp_dir)
        .unwrap()
        .start_should_fail_with_message("failed to open persistent log file")
        .unwrap();
}

#[test]
pub fn fragment_which_reached_mempool_should_be_persisted() {
    let temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump_folder");
    let persistent_log_path = temp_dir.child("persistent_log");
    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallets_having_some_values(vec![&sender, &receiver])
                .with_slots_per_epoch(60.try_into().unwrap())
                .with_slot_duration(3.try_into().unwrap()),
        )
        .with_node_config(NodeConfigBuilder::default().with_mempool(Mempool {
            pool_max_entries: 1usize.into(),
            log_max_entries: 1000usize.into(),
            persistent_log: Some(PersistentLog {
                dir: persistent_log_path.path().to_path_buf(),
            }),
        }))
        .build()
        .start_node(temp_dir)
        .unwrap();

    let adversary_sender = AdversaryFragmentSender::try_from_jormungandr(
        &jormungandr,
        BlockDate::first().next_epoch(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    )
    .unwrap();

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
    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallets_having_some_values(vec![&sender, &receiver])
                .with_slots_per_epoch(60.try_into().unwrap())
                .with_slot_duration(3.try_into().unwrap()),
        )
        .with_node_config(NodeConfigBuilder::default().with_mempool(Mempool {
            pool_max_entries: 1000usize.into(),
            log_max_entries: 1usize.into(),
            persistent_log: Some(PersistentLog {
                dir: persistent_log_path.path().to_path_buf(),
            }),
        }))
        .build()
        .start_node(temp_dir)
        .unwrap();

    let adversary_sender = AdversaryFragmentSender::try_from_jormungandr(
        &jormungandr,
        BlockDate::first().next_epoch(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    )
    .unwrap();

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
    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallets_having_some_values(vec![&sender, &receiver])
                .with_slots_per_epoch(5.try_into().unwrap())
                .with_slot_duration(60.try_into().unwrap()),
        )
        .with_node_config(NodeConfigBuilder::default().with_mempool(Mempool {
            pool_max_entries: 10usize.into(),
            log_max_entries: 10usize.into(),
            persistent_log: Some(PersistentLog {
                dir: persistent_log_path.path().to_path_buf(),
            }),
        }))
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::try_from_with_setup(
        &jormungandr,
        BlockDate::first().next_epoch(),
        FragmentSenderSetup::dump_into(dump_folder.path().to_path_buf()),
    )
    .unwrap();

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
    let mut temp_dir = TempDir::new().unwrap();
    let dump_folder = temp_dir.child("dump_folder");
    let persistent_log_path = temp_dir.child("persistent_log");
    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let config = Block0ConfigurationBuilder::default()
        .with_slots_per_epoch(60.try_into().unwrap())
        .with_slot_duration(3.try_into().unwrap())
        .with_block0_consensus(ConsensusVersion::Bft)
        .with_utxos(vec![
            InitialUTxO {
                address: sender.address(),
                value: 1_000_000.into(),
            },
            InitialUTxO {
                address: receiver.address(),
                value: 1_000_000.into(),
            },
        ]);

    let test_context = SingleNodeTestBootstrapper::default()
        .with_block0_config(config)
        .with_node_config(NodeConfigBuilder::default().with_mempool(Mempool {
            pool_max_entries: 1usize.into(),
            log_max_entries: 1000usize.into(),
            persistent_log: Some(PersistentLog {
                dir: persistent_log_path.path().to_path_buf(),
            }),
        }))
        .as_bft_leader()
        .build();

    let mut jormungandr = test_context.start_node(temp_dir).unwrap();

    let adversary_sender = AdversaryFragmentSender::try_from_jormungandr(
        &jormungandr,
        BlockDate::first().next_epoch(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    )
    .unwrap();

    adversary_sender
        .send_transactions_with_invalid_counter(10, &mut sender, &receiver, &jormungandr)
        .unwrap();

    sleep(Duration::from_secs(1));

    temp_dir = jormungandr.steal_temp_dir().unwrap().try_into().unwrap();

    jormungandr.stop();
    jormungandr = test_context.start_node(temp_dir).unwrap();

    let adversary_sender = AdversaryFragmentSender::try_from_jormungandr(
        &jormungandr,
        BlockDate::first().next_epoch(),
        AdversaryFragmentSenderSetup::dump_into(dump_folder.path().to_path_buf(), false),
    )
    .unwrap();

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
    let temp_dir = TempDir::new().unwrap();

    const N_FRAGMENTS: u32 = 10;

    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallets_having_some_values(vec![&sender, &receiver])
                .with_block_content_max_size(256.into()) // This should only fit 1 transaction
                .with_slots_per_epoch(N_FRAGMENTS.try_into().unwrap()),
        )
        .with_node_config(
            NodeConfigBuilder::default()
                .with_mempool(Mempool {
                    pool_max_entries: 1000.into(),
                    log_max_entries: 1000.into(),
                    persistent_log: None,
                })
                .with_log_level("debug".to_string()),
        )
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::try_from_with_setup(
        &jormungandr,
        BlockDate::first().next_epoch(),
        FragmentSenderSetup::no_verify(),
    )
    .unwrap();

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
    let leader_dir = TempDir::new().unwrap();
    let passive_dir = TempDir::new().unwrap();
    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    let context = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallets_having_some_values(vec![&sender, &receiver])
                .with_block_content_max_size(256.into()) // This should only fit 1 transaction
                .with_slots_per_epoch(N_FRAGMENTS.try_into().unwrap()),
        )
        .with_node_config(
            NodeConfigBuilder::default()
                .with_mempool(Mempool {
                    pool_max_entries: 1000.into(),
                    log_max_entries: 1000.into(),
                    persistent_log: None,
                })
                .with_log_level("debug".to_string()),
        )
        .build();

    let leader = context.start_node(leader_dir).unwrap();

    let passive = JormungandrBootstrapper::default()
        .passive()
        .with_node_config(
            NodeConfigBuilder::default()
                .with_trusted_peers(vec![leader.to_trusted_peer()])
                .with_log_level("debug")
                .build(),
        )
        .with_block0_hash(context.block0_config().to_block_hash())
        .into_starter(passive_dir)
        .unwrap()
        .start()
        .unwrap();

    let fragment_sender = FragmentSender::from_settings_with_setup(
        &passive.rest().settings().unwrap(),
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
    FragmentVerifier::wait_and_verify_is_rejected(Duration::from_secs(30), check, &passive)
        .unwrap();
}

#[test]
/// Verifies `tx_pending` and `mempool_total_size` metrics reported by the node
fn pending_transaction_stats() {
    let bob = thor::Wallet::default();
    let alice = thor::Wallet::default();
    let mempool_max_entries = 1000;
    let temp_dir = TempDir::new().unwrap();

    let leader = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallets_having_some_values(vec![&alice, &bob])
                .with_block_content_max_size(256.into()) // This should only fit 1 transaction)
                .with_slot_duration(30.try_into().unwrap()),
        )
        .with_node_config(
            NodeConfigBuilder::default()
                .with_mempool(Mempool {
                    pool_max_entries: mempool_max_entries.into(),
                    log_max_entries: mempool_max_entries.into(),
                    persistent_log: None,
                })
                .with_log_level("debug".to_string()),
        )
        .build()
        .start_node(temp_dir)
        .unwrap();

    let stats = leader.rest().stats().unwrap().stats.unwrap();

    assert_eq!(stats.mempool_usage_ratio, 0.0);
    assert_eq!(stats.mempool_total_size, 0);

    let fragment_builder = FragmentBuilder::try_from_with_setup(
        &leader,
        BlockDate {
            epoch: 1,
            slot_id: 0,
        },
    )
    .unwrap();

    let mut pending_size = 0;
    let mut pending_cnt = 0;

    for i in 0..10 {
        let transaction = fragment_builder
            .transaction(&alice, bob.address(), i.into())
            .unwrap();

        pending_size += transaction.serialized_size();
        pending_cnt += 1;

        let status =
            FragmentVerifier::fragment_status(leader.send_fragment(transaction).unwrap(), &leader)
                .unwrap();

        let stats = leader.rest().stats().unwrap().stats.unwrap();

        assert!(status.is_pending());
        assert_eq!(
            pending_cnt as f64 / mempool_max_entries as f64,
            stats.mempool_usage_ratio
        );
        assert_eq!(pending_size, stats.mempool_total_size as usize);
    }
}

#[test]
/// Verifies the `block_content_size_avg` metric reported by the node converges under a steady flow
/// of transactions.
fn avg_block_size_stats() {
    const ALICE: &str = "Alice";
    const BOB: &str = "Bob";
    const LEADER: &str = "leader";
    const SLOT_DURATION_SECS: u8 = 1;
    const STABILITY_SLOTS: usize = 3; // Number of slots we expect `block_content_size_avg` to stay the same for
    let linear_fee = LinearFee::new(0, 0, 0);

    let blockchain = BlockchainConfiguration::default()
        .with_slot_duration(SlotDuration::new(SLOT_DURATION_SECS).unwrap())
        .with_linear_fee(linear_fee)
        .with_leader(LEADER)
        .with_block_content_max_size(200.into()); // This should only fit one transaction

    let mut controller = NetworkBuilder::default()
        .blockchain_config(blockchain)
        .topology(Topology::default().with_node(Node::new(LEADER)))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(100_000)
                .discrimination(chain_addr::Discrimination::Test)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(10_000)
                .discrimination(chain_addr::Discrimination::Test)
                .delegated_to(LEADER)
                .build(),
        )
        .build()
        .unwrap();

    let mut alice = controller.controlled_wallet(ALICE).unwrap();
    let bob = controller.controlled_wallet(BOB).unwrap();

    let node = controller.spawn(SpawnParams::new(LEADER).leader()).unwrap();
    let settings = node.rest().settings().unwrap();
    let fragment_sender = FragmentSender::from_settings(
        &settings,
        BlockDateGenerator::rolling(
            &settings,
            BlockDate {
                epoch: 1,
                slot_id: 0,
            },
            false,
        ),
        FragmentSenderSetup::resend_3_times(),
    );

    let mut last_avg = node
        .rest()
        .stats()
        .unwrap()
        .stats
        .unwrap()
        .block_content_size_avg as usize;

    let mut stability_counter = 0;

    for i in 1..60 {
        if stability_counter >= STABILITY_SLOTS {
            return;
        }

        let curr_avg = node
            .rest()
            .stats()
            .unwrap()
            .stats
            .unwrap()
            .block_content_size_avg as usize;

        if last_avg == curr_avg {
            stability_counter += 1;
        } else {
            stability_counter = 0;
        }

        last_avg = curr_avg;

        let check = fragment_sender
            .send_transaction(&mut alice, &bob, &node, i.into())
            .unwrap();

        FragmentVerifier::wait_and_verify_is_in_block(
            Duration::from_secs(SLOT_DURATION_SECS.into()),
            check,
            &node,
        )
        .unwrap();
    }

    panic!("`block_content_size_avg` did not converge");
}
