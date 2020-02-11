use crate::common::{
    jcli_wrapper,
    jormungandr::{starter::Starter, ConfigurationBuilder},
    startup,
};
use jormungandr_lib::interfaces::InitialUTxO;

#[test]
pub fn test_correct_utxos_are_read_from_node() {
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

    let config = ConfigurationBuilder::new().with_funds(funds).build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let rest_addr = jormungandr.rest_address();

    let sender_block0_utxo = config.block0_utxo_for_address(&sender_utxo_address);
    jcli_wrapper::assert_rest_utxo_get_returns_same_utxo(&rest_addr, &sender_block0_utxo);

    let receiver_block0_utxo = config.block0_utxo_for_address(&receiver_utxo_address);
    jcli_wrapper::assert_rest_utxo_get_returns_same_utxo(&rest_addr, &receiver_block0_utxo);
}
