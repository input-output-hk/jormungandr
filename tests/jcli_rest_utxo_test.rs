extern crate assert_cmd;
extern crate galvanic_test;
extern crate mktemp;

mod common;
use common::configuration;
use common::file_assert;
use common::file_utils;
use common::jcli_wrapper;
use common::jormungandr_wrapper;
use common::process_assert;
use common::process_utils;
use configuration::genesis_model::Fund;

#[test]
#[cfg(feature = "integration-test")]
pub fn read_yaml() {
    let sender_private_key = process_utils::run_process_and_get_output_line(
        jcli_wrapper::get_key_generate_command_default(),
    );
    println!("Sender private key generated: {}", &sender_private_key);

    let reciever_private_key = process_utils::run_process_and_get_output_line(
        jcli_wrapper::get_key_generate_command_default(),
    );
    println!("Reciever private key generated: {}", &reciever_private_key);

    let sender_public_key = process_utils::run_process_and_get_output_line(
        jcli_wrapper::get_key_to_public_command(&sender_private_key),
    );
    println!("Sender public key generated: {}", &sender_public_key);

    let reciever_public_key = process_utils::run_process_and_get_output_line(
        jcli_wrapper::get_key_to_public_command(&reciever_private_key),
    );
    println!("Reciever public key generated: {}", &reciever_public_key);

    let sender_address = process_utils::run_process_and_get_output_line(
        jcli_wrapper::get_address_single_command_default(&sender_public_key),
    );
    println!("Sender address generated: {}", &sender_address);

    let reciever_address = process_utils::run_process_and_get_output_line(
        jcli_wrapper::get_address_single_command_default(&reciever_public_key),
    );
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
    let content = serde_yaml::to_string(&genesis_yaml).unwrap();
    let input_yaml_file_path = file_utils::create_file_in_temp("genesis.yaml", &content);

    let response_yaml = process_utils::run_process_and_get_yaml_single(
        jcli_wrapper::get_address_info_command(&sender_address),
    );

    let node_config = configuration::get_node_config_path();
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");

    process_assert::run_and_assert_process_exited_successfully(
        jcli_wrapper::get_genesis_encode_command(&input_yaml_file_path, &path_to_output_block),
        "jcli genesis encode",
    );

    file_assert::assert_file_exists_and_not_empty(&path_to_output_block);
    println!(
        "Created genesis block in: ({:?}) from genesis yaml ({:?}) and node config ({:?})",
        &path_to_output_block, &input_yaml_file_path, &node_config
    );

    let node_config = configuration::get_node_config_path();

    println!("Starting jormungandr node");
    let process = jormungandr_wrapper::start_jormungandr_node(&node_config, &path_to_output_block)
        .spawn()
        .expect("failed to execute 'start jormungandr node'");
    let _guard = process_utils::process_guard::ProcessKillGuard::new(process);

    process_utils::run_process_until_exited_successfully(
        jcli_wrapper::get_rest_stats_command_default(),
        2,
        5,
        "get stats from jormungandr node",
        "jormungandr node is not up",
    );

    let content = process_utils::run_process_and_get_yaml_collection(
        jcli_wrapper::get_rest_utxo_get_command_default(),
    );
    println!("Utxos: {:?}", &content);

    assert_eq!(&funds[0].address, content[0].get("out_addr").unwrap());
    assert_eq!(
        funds[0].value.to_string(),
        content[0].get("out_value").unwrap().to_string()
    );

    assert_eq!(&funds[1].address, content[1].get("out_addr").unwrap());
    assert_eq!(
        funds[1].value.to_string(),
        content[1].get("out_value").unwrap().to_string()
    );
}
