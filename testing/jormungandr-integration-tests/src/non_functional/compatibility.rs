use assert_fs::{fixture::PathChild, TempDir};
use chain_impl_mockchain::block::BlockDate;
use jormungandr_automation::{
    jormungandr::{
        download_last_n_releases, get_jormungandr_bin, ConfigurationBuilder, Starter, Version,
    },
    testing::Release,
};
use jormungandr_lib::interfaces::InitialUTxO;
use thor::{FragmentSender, TransactionHash};

fn test_connectivity_between_master_and_legacy_app(release: Release, temp_dir: &TempDir) {
    println!("Testing version: {}", release.version());

    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();

    let leader_config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(temp_dir);

    let leader_jormungandr = Starter::new()
        .config(leader_config.clone())
        .start()
        .unwrap();

    let trusted_node_config = ConfigurationBuilder::new()
        .with_trusted_peers(vec![leader_jormungandr.to_trusted_peer()])
        .with_block_hash(leader_config.genesis_block_hash())
        .build(temp_dir);

    let trusted_jormungandr = Starter::new()
        .config(trusted_node_config)
        .legacy(release.version())
        .jormungandr_app(get_jormungandr_bin(&release, temp_dir))
        .passive()
        .start()
        .unwrap();

    let new_transaction = thor::FragmentBuilder::new(
        &leader_jormungandr.genesis_block_hash(),
        &leader_jormungandr.fees(),
        BlockDate::first().next_epoch(),
    )
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

    trusted_jormungandr.assert_no_errors_in_log_with_message("newest master has errors in log");
    leader_jormungandr.assert_no_errors_in_log_with_message(&format!(
        "Legacy nodes from {} version, has errrors in logs",
        release.version()
    ));
}

#[test]
// Re-enable when rate of breaking changes subsides and we can maintain
// backward compatible releases again.
#[ignore]
pub fn test_compability() {
    let temp_dir = TempDir::new().unwrap();
    for release in download_last_n_releases(5) {
        test_connectivity_between_master_and_legacy_app(release, &temp_dir);
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

    let mut sender = thor::Wallet::default();
    let mut receiver = thor::Wallet::default();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            sender.to_initial_fund(1_000_000),
            receiver.to_initial_fund(1_000_000),
        ])
        .with_storage(&temp_dir.child("storage"))
        .build(temp_dir);

    // build some storage data on legacy node
    let legacy_jormungandr = Starter::new()
        .config(config.clone())
        .legacy(version.clone())
        .start()
        .unwrap();

    let fragment_sender = FragmentSender::new(
        legacy_jormungandr.genesis_block_hash(),
        legacy_jormungandr.fees(),
        BlockDate::first().next_epoch().into(),
        Default::default(),
    );

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

    // upgrade node to newest

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    fragment_sender
        .send_transactions_round_trip(10, &mut sender, &mut receiver, &jormungandr, 100.into())
        .expect("fragment send error for legacy version");

    jormungandr.assert_no_errors_in_log();
    jormungandr.shutdown();

    // rollback node to legacy again

    let legacy_jormungandr = Starter::new()
        .config(config)
        .legacy(version)
        .start()
        .unwrap();

    let fragment_sender = FragmentSender::new(
        legacy_jormungandr.genesis_block_hash(),
        legacy_jormungandr.fees(),
        BlockDate::first().next_epoch().into(),
        Default::default(),
    );

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
