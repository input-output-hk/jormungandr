use crate::{startup, startup::SingleNodeTestBootstrapper};
use assert_fs::TempDir;
use jormungandr_automation::{
    jcli::JCli, jormungandr::Block0ConfigurationBuilder,
    testing::block0::Block0ConfigurationExtension,
};

#[test]
pub fn test_correct_utxos_are_read_from_node() {
    let jcli: JCli = Default::default();
    let sender_utxo_address = startup::create_new_utxo_address();
    let receiver_utxo_address = startup::create_new_utxo_address();
    let temp_dir = TempDir::new().unwrap();

    let test_context = SingleNodeTestBootstrapper::default()
        .with_block0_config(Block0ConfigurationBuilder::default().with_utxos(vec![
            receiver_utxo_address.to_initial_fund(100),
            sender_utxo_address.to_initial_fund(100),
        ]))
        .as_bft_leader()
        .build();
    let jormungandr = test_context.start_node(temp_dir).unwrap();

    let sender_block0_utxo = test_context
        .block0_config()
        .utxo_for_address(&sender_utxo_address.address());
    let receiver_block0_utxo = test_context
        .block0_config()
        .utxo_for_address(&receiver_utxo_address.address());

    let rest_addr = jormungandr.rest_uri();

    jcli.rest()
        .v0()
        .utxo()
        .assert_contains(&sender_block0_utxo, &rest_addr);

    jcli.rest()
        .v0()
        .utxo()
        .assert_contains(&receiver_block0_utxo, &rest_addr);
}
