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

pub fn assert_genesis_encode_command(
    genesis_yaml_file_path: &PathBuf,
    path_to_output_block: &PathBuf,
) -> () {
    let output = process_utils::run_process_and_get_output(get_genesis_encode_command(
        &genesis_yaml_file_path,
        &path_to_output_block,
    ));

    process_assert::assert_process_exited_successfully(output);
    file_assert::assert_file_exists_and_not_empty(path_to_output_block);

    println!(
        "Created genesis block in: ({:?}) from genesis yaml ({:?}) ",
        &path_to_output_block, &genesis_yaml_file_path
    );
}

/// Get genesis encode command.
///
/// # Arguments
///
/// * `genesis_yaml_fle_path` - Path to genesis yaml file
/// * `path_to_output_block` - Path to output block file
///
fn get_genesis_encode_command(
    genesis_yaml_file_path: &PathBuf,
    path_to_output_block: &PathBuf,
) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("genesis")
        .arg("encode")
        .arg("--input")
        .arg(genesis_yaml_file_path.as_os_str())
        .arg("--output")
        .arg(path_to_output_block.as_os_str());
    command
}

pub fn assert_rest_stats_command_default() -> BTreeMap<String, String> {
    let output = process_utils::run_process_and_get_output(get_rest_stats_command_default());
    let content = output.as_single_node_yaml();
    println!("Returned node info: {:?}", &content);
    process_assert::assert_process_exited_successfully(output);
    content
}

/// Get rest stat command. Uses [default host and port](super::test_const::JORMUNGANDR_ADDRESS)
pub fn get_rest_stats_command_default() -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("rest")
        .arg("v0")
        .arg("node")
        .arg("stats")
        .arg("get")
        .arg("-h")
        .arg(&configuration::JORMUNGANDR_ADDRESS);
    command
}

pub fn assert_rest_utxo_get_command_default() -> Vec<Utxo> {
    let output = process_utils::run_process_and_get_output(get_rest_utxo_get_command_default());
    let content = output.as_lossy_string();
    println!("Returned utxos: {:?}", &content);
    process_assert::assert_process_exited_successfully(output);
    let utxos: Vec<Utxo> = serde_yaml::from_str(&content).unwrap();
    utxos
}

/// Get utxo get command. Uses [default host and port](super::test_const::JORMUNGANDR_ADDRESS)
fn get_rest_utxo_get_command_default() -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("rest")
        .arg("v0")
        .arg("utxo")
        .arg("get")
        .arg("-h")
        .arg(&configuration::JORMUNGANDR_ADDRESS);
    command
}

pub fn assert_get_address_info_command(adress: &str) -> BTreeMap<String, String> {
    let output = process_utils::run_process_and_get_output(get_address_info_command(&adress));
    let content = output.as_single_node_yaml();
    process_assert::assert_process_exited_successfully(output);
    content
}

// Get adress info command.
fn get_address_info_command(adress: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command.arg("address").arg("info").arg(&adress);
    println!("Run address info command: {:?}", &command);
    command
}

pub fn assert_address_single_command_default(public_key: &str) -> String {
    let output =
        process_utils::run_process_and_get_output(get_address_single_command_default(&public_key));
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

/// Get adress single command.
fn get_address_single_command_default(public_key: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("address")
        .arg("single")
        .arg(&public_key)
        .arg("--testing");
    println!("Run address info command: {:?}", &command);
    command
}

pub fn assert_post_transaction_default(transaction_hash: &str) -> () {
    let output = process_utils::run_process_and_get_output(get_post_transaction_command_default(
        &transaction_hash,
    ));
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    assert_eq!("Success!", single_line);
}

/// Get post transaction command. Uses [default host and port](super::test_const::JORMUNGANDR_ADDRESS)
fn get_post_transaction_command_default(transaction_hash: &str) -> Command {
    let transaction_hash_file_path =
        file_utils::create_file_in_temp("spending_key", &transaction_hash);
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("rest")
        .arg("v0")
        .arg("message")
        .arg("post")
        .arg("-f")
        .arg(&transaction_hash_file_path)
        .arg("-h")
        .arg(&configuration::JORMUNGANDR_ADDRESS);
    command
}

pub fn assert_key_generate_command_default() -> String {
    let output = process_utils::run_process_and_get_output(get_key_generate_command_default());
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

/// Get key generate command
fn get_key_generate_command_default() -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    let deafult_extended_key_type = "Ed25519Extended";
    command
        .arg("key")
        .arg("generate")
        .arg("--type")
        .arg(&deafult_extended_key_type);
    command
}

pub fn assert_key_to_public_command_default(private_key: &str) -> String {
    let output = process_utils::run_process_and_get_output(get_key_to_public_command(&private_key));
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

/// Get key to public command
fn get_key_to_public_command(private_key: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    let secret_file_key = file_utils::create_file_in_temp("secret_file_key", &private_key);
    command
        .arg("key")
        .arg("to-public")
        .arg("--input")
        .arg(&secret_file_key);
    command
}
