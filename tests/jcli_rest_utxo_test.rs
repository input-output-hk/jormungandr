extern crate assert_cmd;
extern crate galvanic_test;
extern crate mktemp;

mod common;
use common::configuration;
use common::configuration::genesis_model::Fund;
use common::jcli_wrapper;
use common::jormungandr_wrapper;
use common::startup;

#[test]
#[cfg(feature = "integration-test")]
pub fn test_correct_utxos_are_read_from_node() {
    let sender_private_key = jcli_wrapper::assert_key_generate_default();
    println!("Sender private key generated: {}", &sender_private_key);

    let reciever_private_key = jcli_wrapper::assert_key_generate_default();
    println!("Reciever private key generated: {}", &reciever_private_key);

    let sender_public_key = jcli_wrapper::assert_key_to_public_default(&sender_private_key);
    println!("Sender public key generated: {}", &sender_public_key);

    let reciever_public_key = jcli_wrapper::assert_key_to_public_default(&reciever_private_key);
    println!("Reciever public key generated: {}", &reciever_public_key);

    let sender_address = jcli_wrapper::assert_address_single_default(&sender_public_key);
    println!("Sender address generated: {}", &sender_address);

    let reciever_address = jcli_wrapper::assert_address_single_default(&reciever_public_key);
    println!("Reciever address generated: {}", &reciever_address);

    let funds = vec![
        Fund {
            address: reciever_address.clone(),
            value: 100,
        },
        Fund {
            address: sender_address.clone(),
            value: 100,
        },
    ];

    let genesis_yaml = configuration::genesis_model::GenesisYaml::new_with_funds(funds.clone());
    let node_config = configuration::node_config_model::NodeConfig::new();
    let jormungandr_rest_address = node_config.get_node_address();
    let _jormungandr =
        startup::start_jormungandr_node_with_genesis_conf(&genesis_yaml, &node_config);

    let content = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);

    assert_eq!(content.len(), funds.len());
    assert_eq!(funds[0].address, content[0].out_addr);
    assert_eq!(funds[0].value.to_string(), content[0].out_value.to_string());

    assert_eq!(funds[1].address, content[1].out_addr);
    assert_eq!(funds[1].value.to_string(), content[1].out_value.to_string());
}
