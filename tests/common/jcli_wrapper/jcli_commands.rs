use super::configuration;
use super::file_utils;
use std::path::PathBuf;
use std::process::Command;

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

/// Get genesis hash command.
///
/// # Arguments
///
/// * `path_to_output_block` - Path to output block file
///
pub fn get_genesis_hash_command(path_to_output_block: &PathBuf) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("genesis")
        .arg("hash")
        .arg("--input")
        .arg(path_to_output_block.as_os_str());
    command
}

/// Get rest stat command. Uses [default host and port](super::test_const::JORMUNGANDR_ADDRESS)
pub fn get_rest_stats_command(host: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("rest")
        .arg("v0")
        .arg("node")
        .arg("stats")
        .arg("get")
        .arg("-h")
        .arg(&host);
    command
}

/// Get utxo get command. Uses [default host and port](super::test_const::JORMUNGANDR_ADDRESS)
pub fn get_rest_utxo_get_command(host: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("rest")
        .arg("v0")
        .arg("utxo")
        .arg("get")
        .arg("-h")
        .arg(&host);
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

/// Get adress single command.
pub fn get_address_info_command_default(address: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command.arg("address").arg("info").arg(&address);
    println!("Run address info command: {:?}", &command);
    command
}

/// Get adress single command.
pub fn get_address_account_command_default(public_key: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("address")
        .arg("account")
        .arg(&public_key)
        .arg("--testing");
    println!("Run address info command: {:?}", &command);
    command
}

/// Get adress single command.
pub fn get_address_delegation_command_default(public_key: &str, delegation_key: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("address")
        .arg("singl")
        .arg(&public_key)
        .arg(&delegation_key)
        .arg("--testing");
    println!("Run address info command: {:?}", &command);
    command
}

/// Get post transaction command.
pub fn get_post_transaction_command(transaction_hash: &str, host: &str) -> Command {
    let transaction_hash_file_path =
        file_utils::create_file_in_temp("transaction.hash", &transaction_hash);
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("rest")
        .arg("v0")
        .arg("message")
        .arg("post")
        .arg("-f")
        .arg(&transaction_hash_file_path)
        .arg("-h")
        .arg(&host);
    command
}

/// Get key generate command
pub fn get_key_generate_command_default() -> Command {
    let deafult_extended_key_type = "Ed25519Extended";
    let mut command = get_key_generate_command(&deafult_extended_key_type);
    command
}

/// Get key generate command
pub fn get_key_generate_command(key_type: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("key")
        .arg("generate")
        .arg("--type")
        .arg(&key_type);
    command
}

/// Get key generate command
pub fn get_key_generate_with_seed_command(key_type: &str, seed: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("key")
        .arg("generate")
        .arg("--type")
        .arg(&key_type)
        .arg("--seed")
        .arg(&seed);
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

/// Get key to public command
pub fn get_key_to_bytes_command(input_file: &PathBuf, output_file: &PathBuf) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("key")
        .arg("to-bytes")
        .arg(output_file.as_os_str())
        .arg(input_file.as_os_str());
    command
}

pub fn get_key_from_bytes_command(input_file: &PathBuf, key_type: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app().as_os_str());
    command
        .arg("key")
        .arg("from-bytes")
        .arg(input_file.as_os_str())
        .arg("--type")
        .arg(&key_type);
    command
}
