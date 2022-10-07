use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use chain_impl_mockchain::fee::LinearFee;
use jormungandr_automation::{
    jcli::JCli, jormungandr::Block0ConfigurationBuilder,
    testing::block0::Block0ConfigurationExtension,
};
use jormungandr_lib::interfaces::BlockDate;
use std::io::Write;
use thor::Block0ConfigurationBuilderExtension;

#[test]
pub fn test_make_test_transaction() {
    let temp_dir = TempDir::new().unwrap();

    let jcli: JCli = Default::default();
    let sender = thor::Wallet::default();

    let sk_file_path = temp_dir.join("sender.sk");

    {
        let mut sk_file = std::fs::File::create(&sk_file_path).unwrap();
        sk_file
            .write_all(sender.signing_key_to_string().as_bytes())
            .unwrap();
    }

    let staging_file = temp_dir.join("staging.txt");

    let test_context = SingleNodeTestBootstrapper::default()
        .with_block0_config(Block0ConfigurationBuilder::default().with_wallet(&sender, 100.into()))
        .as_bft_leader()
        .build();
    let jormungandr = test_context.start_node(temp_dir).unwrap();
    let config = test_context.block0_config();

    jcli.transaction().make_transaction(
        jormungandr.rest_uri(),
        sender.address(),
        None,
        100.into(),
        config.to_block_hash().to_string(),
        BlockDate::new(1, 0),
        sk_file_path,
        staging_file,
        false,
    );
}

#[test]
pub fn test_make_transaction_to_receiver_account() {
    let temp_dir = TempDir::new().unwrap();

    let jcli: JCli = Default::default();
    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();

    let sk_file_path = temp_dir.join("sender.sk");

    {
        let mut sk_file = std::fs::File::create(&sk_file_path).unwrap();
        sk_file
            .write_all(sender.signing_key_to_string().as_bytes())
            .unwrap();
    }

    let staging_file = temp_dir.join("staging.txt");

    let test_context = SingleNodeTestBootstrapper::default()
        .with_block0_config(Block0ConfigurationBuilder::default().with_wallet(&sender, 100.into()))
        .as_bft_leader()
        .build();
    let jormungandr = test_context.start_node(temp_dir).unwrap();
    let config = test_context.block0_config();

    jcli.transaction().make_transaction(
        jormungandr.rest_uri(),
        sender.address(),
        Some(receiver.address()),
        100.into(),
        config.to_block_hash().to_string(),
        BlockDate::new(1, 0),
        sk_file_path,
        staging_file,
        false,
    );
}

#[test]
pub fn test_make_transaction_to_receiver_account_with_fees() {
    let temp_dir = TempDir::new().unwrap();

    let jcli: JCli = Default::default();
    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();

    let sk_file_path = temp_dir.join("sender.sk");

    {
        let mut sk_file = std::fs::File::create(&sk_file_path).unwrap();
        sk_file
            .write_all(sender.signing_key_to_string().as_bytes())
            .unwrap();
    }

    let staging_file = temp_dir.join("staging.txt");

    let config = Block0ConfigurationBuilder::default()
        .with_wallet(&sender, 111.into())
        .with_linear_fees(LinearFee::new(10, 0, 0));
    let test_context = SingleNodeTestBootstrapper::default()
        .with_block0_config(config)
        .as_bft_leader()
        .build();
    let jormungandr = test_context.start_node(temp_dir).unwrap();
    let config = test_context.block0_config();

    jcli.transaction().make_transaction(
        jormungandr.rest_uri(),
        sender.address(),
        Some(receiver.address()),
        100.into(),
        config.to_block_hash().to_string(),
        BlockDate::new(1, 0),
        sk_file_path,
        staging_file,
        false,
    );
}

#[test]
pub fn test_make_transaction_to_receiver_account_with_fees_and_post() {
    let temp_dir = TempDir::new().unwrap();

    let jcli: JCli = Default::default();
    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();

    let sk_file_path = temp_dir.join("sender.sk");

    {
        let mut sk_file = std::fs::File::create(&sk_file_path).unwrap();
        sk_file
            .write_all(sender.signing_key_to_string().as_bytes())
            .unwrap();
    }

    let staging_file = temp_dir.join("staging.txt");

    let config = Block0ConfigurationBuilder::default()
        .with_wallet(&sender, 111.into())
        .with_linear_fees(LinearFee::new(10, 0, 0));
    let test_context = SingleNodeTestBootstrapper::default()
        .with_block0_config(config)
        .as_bft_leader()
        .build();
    let jormungandr = test_context.start_node(temp_dir).unwrap();
    let config = test_context.block0_config();

    jcli.transaction().make_transaction(
        jormungandr.rest_uri(),
        sender.address(),
        Some(receiver.address()),
        100.into(),
        config.to_block_hash().to_string(),
        BlockDate::new(1, 0),
        sk_file_path,
        staging_file,
        true,
    );
}
