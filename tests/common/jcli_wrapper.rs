use std::path::PathBuf;
use std::process::Command;

use super::configuration;
use super::file_utils;

/// Get genesis encode command.
///
/// # Arguments
///
/// * `genesis_yaml_fle_path` - Path to genesis yaml file
/// * `path_to_output_block` - Path to output block file
///
pub fn get_genesis_encode_command(
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

/// Get utxo get command. Uses [default host and port](super::test_const::JORMUNGANDR_ADDRESS)
pub fn get_rest_utxo_get_command_default() -> Command {
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

/// Get transaction build command
pub fn get_transaction_build_get_command(
    input_tx: &str,
    ouput_adress: &str,
    spending_key: &str,
) -> Command {
    let spending_key_file_path = file_utils::create_file_in_temp("spending_key", &spending_key);
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("transaction")
        .arg("build")
        .arg("--input")
        .arg(&input_tx)
        .arg("--output")
        .arg(&ouput_adress)
        .arg("-s")
        .arg(&spending_key_file_path.as_os_str());
    println!("{:?}", &command);
    command
}

/// Get adress info command.
pub fn get_address_info_command(adress: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command.arg("address").arg("info").arg(&adress);
    println!("Run address info command: {:?}", &command);
    command
}

/// Get adress single command.
pub fn get_address_single_command_default(public_key: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("address")
        .arg("single")
        .arg(&public_key)
        .arg("--testing");
    println!("Run address info command: {:?}", &command);
    command
}

/// Get post transaction command. Uses [default host and port](super::test_const::JORMUNGANDR_ADDRESS)
pub fn get_post_transaction_command_default(transaction_hash: &str) -> Command {
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

/// Get key generate command
pub fn get_key_generate_command_default() -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    let deafult_extended_key_type = "Ed25519Extended";
    command
        .arg("key")
        .arg("generate")
        .arg("--type")
        .arg(&deafult_extended_key_type);
    command
}

/// Get key to public command
pub fn get_key_to_public_command(private_key: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    let secret_file_key = file_utils::create_file_in_temp("secret_file_key", &private_key);
    command
        .arg("key")
        .arg("to-public")
        .arg("--input")
        .arg(&secret_file_key);
    command
}
