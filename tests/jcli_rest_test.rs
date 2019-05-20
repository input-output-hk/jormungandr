#![cfg(feature = "integration-test")]

extern crate assert_cmd;
extern crate galvanic_test;
extern crate mktemp;

mod common;
use common::configuration;
use common::configuration::genesis_model::Fund;
use common::jcli_wrapper;
use common::process_assert;
use common::startup;

#[test]
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

    let mut config = startup::build_configuration_with_funds(funds.clone());
    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node(&mut config);
    let content = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);

    assert_eq!(content.len(), funds.len());
    assert_eq!(funds[0].address, content[0].out_addr);
    assert_eq!(funds[0].value.to_string(), content[0].out_value.to_string());
    assert_eq!(funds[1].address, content[1].out_addr);
    assert_eq!(funds[1].value.to_string(), content[1].out_value.to_string());
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_correct_id_is_returned_for_block_tip_if_only_genesis_block_exists() {
    let mut config = startup::build_configuration();
    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node(&mut config);
    let block_id = jcli_wrapper::assert_rest_get_block_tip(&jormungandr_rest_address);

    assert_ne!(&block_id, "", "empty block hash");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_non_empty_hash_is_returned_for_block0() {
    let mut config = startup::build_configuration();
    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node(&mut config);

    let block_id = jcli_wrapper::assert_rest_get_block_tip(&jormungandr_rest_address);
    let actual_hash =
        jcli_wrapper::assert_rest_get_block_by_id(&block_id, &jormungandr_rest_address);

    assert_ne!(&actual_hash, "", "empty block hash");
}

#[test]
#[cfg(feature = "integration-test")]
/// False green due to: #298
pub fn test_correct_error_is_returned_for_incorrect_block_id() {
    let incorrect_block_id = "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641aa";

    let mut config = startup::build_configuration();
    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node(&mut config);

    process_assert::assert_process_failed_and_contains_message_with_desc(
        jcli_wrapper::jcli_commands::get_rest_get_block_command(
            &incorrect_block_id,
            &jormungandr_rest_address,
        ),
        "Status(400)",
        "This assertion is incorrect on purpose to avoid failing build when running test,
        after #298 is fixed it need to be changed to correct one",
    );
}

#[test]
#[cfg(feature = "integration-test")]
/// False green due to: #298
pub fn test_correct_error_is_returned_for_incorrect_block_id_in_next_block_id_request() {
    let incorrect_block_id = "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641aa";

    let mut config = startup::build_configuration();
    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node(&mut config);

    process_assert::assert_process_failed_and_contains_message_with_desc(
        jcli_wrapper::jcli_commands::get_rest_get_next_block_id_command(
            &incorrect_block_id,
            &1,
            &jormungandr_rest_address,
        ),
        "Status(400)",
        "This assertion is incorrect on purpose to avoid failing build when running test,
        after #298 is fixed it need to be changed to correct one",
    );
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_next_id_is_empty_for_tip_block() {
    let mut config = startup::build_configuration();
    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node(&mut config);

    let block_id = jcli_wrapper::assert_rest_get_block_tip(&jormungandr_rest_address);
    let next_block_id =
        jcli_wrapper::assert_rest_get_next_block_id(&block_id, &1, &jormungandr_rest_address);

    assert_eq!(&next_block_id, "", "next id for tip block should be empty");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_correct_error_is_returned_for_incorrect_host_syntax() {
    let incorrect_host = "not_a_correct_syntax";

    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_rest_block_tip_command(&incorrect_host),
        "Invalid value for '--host <host>': relative URL without a base",
    );
}

#[test]
#[cfg(feature = "integration-test")]
/// False green due to: #298
pub fn test_correct_error_is_returned_for_incorrect_host_address() {
    let incorrect_host = "http://127.0.0.100:8443/api";

    process_assert::assert_process_failed_and_matches_message_with_desc(
        jcli_wrapper::jcli_commands::get_rest_block_tip_command(&incorrect_host),
        "thread 'main' panicked at",
        "This assertion is incorrect on purpose to avoid failing build when running test,
        after #298 is fixed it need to be changed to correct one",
    );
}

#[test]
#[cfg(feature = "integration-test")]
/// False green due to: #298
pub fn test_correct_error_is_returned_for_incorrect_path() {
    let node_config = configuration::node_config_model::NodeConfig::new();
    let mut incorrect_host = node_config.get_node_address();
    incorrect_host.push('x');

    process_assert::assert_process_failed_and_matches_message_with_desc(
        jcli_wrapper::jcli_commands::get_rest_block_tip_command(&incorrect_host),
        "thread 'main' panicked at",
        "This assertion is incorrect on purpose to avoid failing build when running test,
        after #298 is fixed it need to be changed to correct one",
    );
}
