use jormungandr_testing_utils::testing::common::{
    jcli::JCli,
    jormungandr::{starter::Starter, ConfigurationBuilder},
    startup,
};
use jormungandr_lib::interfaces::InitialUTxO;

use assert_fs::TempDir;

#[test]
pub fn test_correct_utxos_are_read_from_node() {
    let jcli: JCli = Default::default();
    let sender_utxo_address = startup::create_new_utxo_address();
    let receiver_utxo_address = startup::create_new_utxo_address();

    let funds = vec![
        InitialUTxO {
            address: receiver_utxo_address.address(),
            value: 100.into(),
        },
        InitialUTxO {
            address: sender_utxo_address.address(),
            value: 100.into(),
        },
    ];

    let temp_dir = TempDir::new().unwrap();

    let config = ConfigurationBuilder::new()
        .with_funds(funds)
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let rest_addr = jormungandr.rest_uri();

    let sender_block0_utxo = config.block0_utxo_for_address(&sender_utxo_address);
    jcli.rest()
        .v0()
        .utxo()
        .assert_contains(&sender_block0_utxo, &rest_addr);

    let receiver_block0_utxo = config.block0_utxo_for_address(&receiver_utxo_address);
    jcli.rest()
        .v0()
        .utxo()
        .assert_contains(&receiver_block0_utxo, &rest_addr);
}
