use crate::common::{
    configuration::genesis_model::Fund,
    data::address::Utxo,
    jcli_wrapper,
    jormungandr::{starter::Starter, ConfigurationBuilder},
};
use chain_addr::Discrimination;

#[test]
pub fn test_correct_utxos_are_read_from_node() {
    let sender_private_key = jcli_wrapper::assert_key_generate_default();
    println!("Sender private key generated: {}", &sender_private_key);

    let receiver_private_key = jcli_wrapper::assert_key_generate_default();
    println!("Receiver private key generated: {}", &receiver_private_key);

    let sender_public_key = jcli_wrapper::assert_key_to_public_default(&sender_private_key);
    println!("Sender public key generated: {}", &sender_public_key);

    let receiver_public_key = jcli_wrapper::assert_key_to_public_default(&receiver_private_key);
    println!("Receiver public key generated: {}", &receiver_public_key);

    let sender_address =
        jcli_wrapper::assert_address_single(&sender_public_key, Discrimination::Test);
    println!("Sender address generated: {}", &sender_address);

    let receiver_address =
        jcli_wrapper::assert_address_single(&receiver_public_key, Discrimination::Test);
    println!("Receiver address generated: {}", &receiver_address);

    let sender_utxo_address = Utxo {
        private_key: sender_private_key.clone(),
        public_key: sender_public_key.clone(),
        address: sender_address.clone(),
    };

    let receiver_utxo_address = Utxo {
        private_key: receiver_private_key.clone(),
        public_key: receiver_public_key.clone(),
        address: receiver_address.clone(),
    };

    let funds = vec![
        Fund {
            address: receiver_address.clone(),
            value: 100.into(),
        },
        Fund {
            address: sender_address.clone(),
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
