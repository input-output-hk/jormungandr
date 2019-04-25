pub mod jcli_commands;
pub mod jcli_transaction_wrapper;
pub mod utxo;
use self::utxo::Utxo;
use super::configuration;
use super::file_assert;
use super::file_utils;
use super::process_assert;
use super::process_utils;
use super::process_utils::output_extensions::ProcessOutput;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

pub fn assert_genesis_encode(
    genesis_yaml_file_path: &PathBuf,
    path_to_output_block: &PathBuf,
) -> () {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_genesis_encode_command(&genesis_yaml_file_path, &path_to_output_block),
    );

    process_assert::assert_process_exited_successfully(output);
    file_assert::assert_file_exists_and_not_empty(path_to_output_block);

    println!(
        "Created genesis block in: ({:?}) from genesis yaml ({:?}) ",
        &path_to_output_block, &genesis_yaml_file_path
    );
}

pub fn assert_rest_stats(host: &str) -> BTreeMap<String, String> {
    let output =
        process_utils::run_process_and_get_output(jcli_commands::get_rest_stats_command(&host));
    let content = output.as_single_node_yaml();
    println!("Returned node info: {:?}", &content);
    process_assert::assert_process_exited_successfully(output);
    content
}
pub fn assert_rest_utxo_get(host: &str) -> Vec<Utxo> {
    let output =
        process_utils::run_process_and_get_output(jcli_commands::get_rest_utxo_get_command(&host));
    let content = output.as_lossy_string();
    println!("Returned utxos: {:?}", &content);
    process_assert::assert_process_exited_successfully(output);
    let utxos: Vec<Utxo> = serde_yaml::from_str(&content).unwrap();
    utxos
}

pub fn assert_get_address_info(adress: &str) -> BTreeMap<String, String> {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_address_info_command_default(&adress),
    );
    let content = output.as_single_node_yaml();
    process_assert::assert_process_exited_successfully(output);
    content
}

pub fn assert_address_single_default(public_key: &str) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_address_single_command_default(&public_key),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_post_transaction(transaction_hash: &str, host: &str) -> () {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_post_transaction_command(&transaction_hash, &host),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    assert_eq!("Success!", single_line);
}

pub fn assert_key_generate_default() -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_key_generate_command_default(),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_key_generate(key_type: &str) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_key_generate_command(&key_type),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_key_with_seed_generate(key_type: &str, seed: &str) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_key_generate_with_seed_command(&key_type, &seed),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_key_to_public_default(private_key: &str) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_key_to_public_command(&private_key),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_key_to_bytes(private_key: &str, path_to_output_file: &PathBuf) -> () {
    let input_file = file_utils::create_file_in_temp("input_key_to_bytes", &private_key);

    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_key_to_bytes_command(&input_file, &path_to_output_file),
    );
    process_assert::assert_process_exited_successfully(output);
}

pub fn assert_key_from_bytes(path_to_input_file: &PathBuf, key_type: &str) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_key_from_bytes_command(&path_to_input_file, &key_type),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}
