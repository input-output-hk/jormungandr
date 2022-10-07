use crate::startup::LegacySingleNodeTestBootstrapper;
use assert_fs::{fixture::PathChild, TempDir};
use chain_impl_mockchain::block::BlockDate;
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{BlockchainConfiguration, SessionSettings, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::{
    jormungandr::{
        download_last_n_releases, get_jormungandr_bin, Block0ConfigurationBuilder,
        JormungandrBootstrapper, LegacyNodeConfigBuilder, NodeConfigBuilder, Version,
    },
    testing::Release,
};
use thor::{FragmentSender, TransactionHash};

const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";
const ALICE: &str = "ALICE";

fn test_connectivity_between_master_and_legacy_app(release: Release) {
    println!("Testing version: {}", release.version());

    let releases = download_last_n_releases(1);
    let last_release = releases.last().unwrap();
    let session_settings = SessionSettings::default();
    let legacy_app = get_jormungandr_bin(last_release, &session_settings.root.child("jormungandr"));

    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_1))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1)),
        )
        .blockchain_config(
            BlockchainConfiguration::default().with_leaders(vec![LEADER_1, LEADER_2]),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .build()
        .unwrap();

    let leader_jormungandr = controller
        .spawn(SpawnParams::new(LEADER_1).leader())
        .unwrap();

    let passive_jormungandr = controller
        .spawn(
            SpawnParams::new(LEADER_2)
                .passive()
                .jormungandr(legacy_app)
                .version(last_release.version()),
        )
        .unwrap();

    let sender = controller.controlled_wallet(ALICE).unwrap();
    let receiver = thor::Wallet::default();

    let new_transaction = thor::FragmentBuilder::try_from_with_setup(
        &leader_jormungandr,
        BlockDate::first().next_epoch(),
    )
    .unwrap()
    .transaction(&sender, receiver.address(), 1.into())
    .unwrap()
    .encode();

    let message = format!(
        "Unable to connect newest master with node from {} version",
        release.version()
    );
    assert!(
        super::check_transaction_was_processed(new_transaction, &receiver, 1, &leader_jormungandr)
            .is_ok(),
        "{}",
        message
    );

    leader_jormungandr.assert_no_errors_in_log_with_message("newest master has errors in log");
    passive_jormungandr.assert_no_errors_in_log_with_message(&format!(
        "Legacy nodes from {} version, has errrors in logs",
        release.version()
    ));
}

#[test]
// Re-enable when rate of breaking changes subsides and we can maintain
// backward compatible releases again.
#[ignore]
pub fn test_compability() {
    for release in download_last_n_releases(5) {
        test_connectivity_between_master_and_legacy_app(release);
    }
}

#[test]
pub fn test_upgrade_downgrade() {
    let temp_dir = TempDir::new().unwrap();
    for release in download_last_n_releases(1) {
        test_upgrade_and_downgrade_from_legacy_to_master(release.version(), &temp_dir);
    }
}

fn test_upgrade_and_downgrade_from_legacy_to_master(version: Version, temp_dir: &TempDir) {
    println!("Testing version: {}", version);
    let node_temp_dir = TempDir::new().unwrap();

    let mut sender = thor::Wallet::default();
    let mut receiver = thor::Wallet::default();

    let storage = temp_dir.child("storage").to_path_buf();

    let legacy_test_context = LegacySingleNodeTestBootstrapper::from(version)
        .as_bft_leader()
        .with_block0_config(Block0ConfigurationBuilder::default().with_utxos(vec![
            sender.to_initial_fund(1_000_000),
            receiver.to_initial_fund(1_000_000),
        ]))
        .with_node_config(LegacyNodeConfigBuilder::default().with_storage(storage.clone()))
        .build()
        .unwrap();

    // build some storage data on legacy node
    let mut legacy_jormungandr = legacy_test_context.start_node(node_temp_dir).unwrap();

    let fragment_sender = FragmentSender::from(&legacy_jormungandr.rest().settings().unwrap());

    fragment_sender
        .send_transactions_round_trip(
            10,
            &mut sender,
            &mut receiver,
            &legacy_jormungandr,
            100.into(),
        )
        .expect("fragment send error for legacy version");

    legacy_jormungandr.assert_no_errors_in_log();

    legacy_jormungandr.shutdown();

    let temp_dir = legacy_jormungandr
        .steal_temp_dir()
        .unwrap()
        .try_into()
        .unwrap();
    // upgrade node to newest

    let jormungandr = JormungandrBootstrapper::default()
        .with_block0_configuration(legacy_test_context.block0_config())
        .with_node_config(NodeConfigBuilder::default().with_storage(storage).build())
        .with_secret(legacy_test_context.test_context.secret_factory.clone())
        .start(temp_dir)
        .unwrap();

    fragment_sender
        .send_transactions_round_trip(10, &mut sender, &mut receiver, &jormungandr, 100.into())
        .expect("fragment send error for legacy version");

    jormungandr.assert_no_errors_in_log();
    jormungandr.shutdown();

    // rollback node to legacy again
    let temp_dir = legacy_jormungandr
        .steal_temp_dir()
        .unwrap()
        .try_into()
        .unwrap();

    let legacy_jormungandr = JormungandrBootstrapper::default()
        .with_block0_configuration(legacy_test_context.block0_config())
        .with_legacy_node_config(legacy_test_context.legacy_node_config)
        .with_secret(legacy_test_context.test_context.secret_factory)
        .start(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::from(&legacy_jormungandr.rest().settings().unwrap());

    fragment_sender
        .send_transactions_round_trip(
            1,
            &mut sender,
            &mut receiver,
            &legacy_jormungandr,
            100.into(),
        )
        .expect("fragment send error for legacy version");

    legacy_jormungandr.assert_no_errors_in_log();
    legacy_jormungandr.shutdown();
}
