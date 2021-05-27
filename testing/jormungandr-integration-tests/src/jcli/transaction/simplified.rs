use crate::common::jormungandr::ConfigurationBuilder;
use crate::common::{jcli::JCli, jormungandr::starter::Starter, startup};
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::InitialUTxO;

use assert_fs::TempDir;
use std::io::Write;

#[test]
pub fn test_make_test_transaction() {
    let temp_dir = TempDir::new().unwrap();

    let jcli: JCli = Default::default();
    let sender = startup::create_new_account_address();

    let sk_file_path = temp_dir.join("sender.sk");

    {
        let mut sk_file = std::fs::File::create(&sk_file_path).unwrap();
        sk_file
            .write_all(sender.signing_key_to_string().as_bytes())
            .unwrap();
    }

    let staging_file = temp_dir.join("staging.txt");

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();

    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();

    jcli.transaction().make_transaction(
        jormungandr.rest_uri(),
        sender.address(),
        100.into(),
        block0_hash.to_string(),
        sk_file_path,
        staging_file,
    );
}
