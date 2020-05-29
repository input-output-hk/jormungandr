use crate::common::{
    jormungandr::{ConfigurationBuilder, Starter},
    legacy::{download_last_n_releases, Version},
    startup,
    transaction_utils::TransactionHash,
};
use assert_fs::TempDir;
use jormungandr_lib::interfaces::InitialUTxO;
use std::str::FromStr;

fn test_connectivity_between_master_and_legacy_app(version: String, temp_dir: &TempDir) {
    println!("Testing version: {}", version);

    let mut sender = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();

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
        .with_block_hash(leader_config.genesis_block_hash().clone())
        .build(temp_dir);

    let version = Version::from_str(&version).unwrap();

    let trusted_jormungandr = Starter::new()
        .config(trusted_node_config)
        .legacy(version)
        .passive()
        .start()
        .unwrap();

    let new_transaction = sender
        .transaction_to(
            &leader_jormungandr.genesis_block_hash(),
            &leader_jormungandr.fees(),
            receiver.address(),
            1.into(),
        )
        .unwrap()
        .encode();

    let message = format!(
        "Unable to connect newest master with node from {} version",
        version
    );
    assert!(
        super::check_transaction_was_processed(new_transaction, &receiver, 1, &leader_jormungandr)
            .is_ok(),
        message
    );

    trusted_jormungandr.assert_no_errors_in_log_with_message("newest master has errors in log");
    leader_jormungandr.assert_no_errors_in_log_with_message(&format!(
        "Legacy nodes from {} version, has errrors in logs",
        version
    ));
}

#[test]
pub fn test_compability() {
    let temp_dir = TempDir::new().unwrap();
    for release in download_last_n_releases(5) {
        test_connectivity_between_master_and_legacy_app(release.version(), &temp_dir);
    }
}
