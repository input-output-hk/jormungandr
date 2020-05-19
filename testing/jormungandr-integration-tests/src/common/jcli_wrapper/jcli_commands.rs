#![allow(dead_code)]

use super::configuration;
use super::Discrimination;

use std::path::Path;
use std::process::Command;

/// Get genesis encode command.
///
/// # Arguments
///
/// * `genesis_yaml_fle_path` - Path to genesis yaml file
/// * `path_to_output_block` - Path to output block file
///
pub fn get_genesis_encode_command(
    genesis_yaml_file_path: &Path,
    path_to_output_block: &Path,
) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("genesis")
        .arg("encode")
        .arg("--input")
        .arg(genesis_yaml_file_path)
        .arg("--output")
        .arg(path_to_output_block);
    command
}

pub fn get_genesis_decode_command(
    genesis_yaml_file_path: &Path,
    path_to_output_block: &Path,
) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("genesis")
        .arg("decode")
        .arg("--input")
        .arg(genesis_yaml_file_path)
        .arg("--output")
        .arg(path_to_output_block);
    command
}

/// Get genesis hash command.
///
/// # Arguments
///
/// * `path_to_output_block` - Path to output block file
///
pub fn get_genesis_hash_command(path_to_output_block: &Path) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("genesis")
        .arg("hash")
        .arg("--input")
        .arg(path_to_output_block);
    command
}

/// Get rest stat command.
pub fn get_rest_stats_command(host: &str) -> Command {
    let mut command = get_jcli_command();
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

/// Get rest stat command.
pub fn get_rest_shutdown_command(host: &str) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("rest")
        .arg("v0")
        .arg("shutdown")
        .arg("get")
        .arg("-h")
        .arg(&host);
    command
}

/// Get rest stat command.
pub fn get_rest_account_stats_command(address: &str, host: &str) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("rest")
        .arg("v0")
        .arg("account")
        .arg("get")
        .arg(&address)
        .arg("-h")
        .arg(&host);
    command
}

/// Get rest block tip command.
pub fn get_rest_block_tip_command(host: &str) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("rest")
        .arg("v0")
        .arg("tip")
        .arg("get")
        .arg("-h")
        .arg(&host);
    command
}

pub fn get_rest_leaders_logs_command(host: &str) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("rest")
        .arg("v0")
        .arg("leaders")
        .arg("logs")
        .arg("get")
        .arg("-h")
        .arg(&host);
    command
}

/// Get rest block command.
pub fn get_rest_get_block_command(block_id: &str, host: &str) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("rest")
        .arg("v0")
        .arg("block")
        .arg(&block_id)
        .arg("get")
        .arg("-h")
        .arg(&host);
    command
}

/// Get rest next block id command.
pub fn get_rest_get_next_block_id_command(block_id: &str, id_count: i32, host: &str) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("rest")
        .arg("v0")
        .arg("block")
        .arg(&block_id)
        .arg("next-id")
        .arg("get")
        .arg("--count")
        .arg(id_count.to_string())
        .arg("-h")
        .arg(&host);
    command
}

/// Get utxo get command.
pub fn get_rest_utxo_get_command(host: &str, fragment_id_hex: &str, output_index: u8) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("rest")
        .arg("v0")
        .arg("utxo")
        .arg(fragment_id_hex)
        .arg(output_index.to_string())
        .arg("get")
        .arg("-h")
        .arg(&host);
    command
}

/// Get address single command.
pub fn get_address_single_command(public_key: &str, discrimination: Discrimination) -> Command {
    let mut command = get_jcli_command();
    command.arg("address").arg("single").arg(&public_key);
    add_discrimination(&mut command, discrimination);
    command
}

/// Get address single command.
pub fn get_address_info_command_default(address: &str) -> Command {
    let mut command = get_jcli_command();
    command.arg("address").arg("info").arg(&address);
    command
}

/// Get address single command.
pub fn get_address_account_command(public_key: &str, discrimination: Discrimination) -> Command {
    let mut command = get_jcli_command();
    command.arg("address").arg("account").arg(&public_key);
    add_discrimination(&mut command, discrimination);
    command
}

fn add_discrimination(command: &mut Command, discrimination: Discrimination) {
    if discrimination == Discrimination::Test {
        command.arg("--testing");
    }
}

/// Get address single command.
pub fn get_genesis_init_command() -> Command {
    let mut command = get_jcli_command();
    command.arg("genesis").arg("init");
    command
}

/// Get address single command.
pub fn get_address_delegation_command(
    public_key: &str,
    delegation_key: &str,
    discrimination: Discrimination,
) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("address")
        .arg("single")
        .arg(&public_key)
        .arg(&delegation_key);
    add_discrimination(&mut command, discrimination);
    println!("Run address info command: {:?}", &command);
    command
}

/// Get post transaction command.
pub fn get_post_transaction_command(transaction_hash_file: &Path, host: &str) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("rest")
        .arg("v0")
        .arg("message")
        .arg("post")
        .arg("-f")
        .arg(transaction_hash_file)
        .arg("-h")
        .arg(&host);
    command
}

/// Get key generate command
pub fn get_key_generate_command_default() -> Command {
    let default_key_type = "Ed25519Extended";
    get_key_generate_command(&default_key_type)
}

/// Get key generate command
pub fn get_key_generate_command(key_type: &str) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("key")
        .arg("generate")
        .arg("--type")
        .arg(&key_type);
    command
}

/// Get key generate command
pub fn get_key_generate_with_seed_command(key_type: &str, seed: &str) -> Command {
    let mut command = get_jcli_command();
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
pub fn get_key_to_public_command(secret_key_file: &Path) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("key")
        .arg("to-public")
        .arg("--input")
        .arg(secret_key_file);
    command
}

/// Get key to public command
pub fn get_key_to_bytes_command(input_file: &Path, output_file: &Path) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("key")
        .arg("to-bytes")
        .arg(output_file)
        .arg(input_file);
    command
}

pub fn get_key_from_bytes_command(input_file: &Path, key_type: &str) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("key")
        .arg("from-bytes")
        .arg(input_file)
        .arg("--type")
        .arg(&key_type);
    command
}

pub fn get_rest_message_log_command(host: &str) -> Command {
    let mut command = get_jcli_command();
    command
        .arg("rest")
        .arg("v0")
        .arg("message")
        .arg("logs")
        .arg("--host")
        .arg(&host);
    command
}

fn get_jcli_command() -> Command {
    let mut command = Command::new(configuration::get_jcli_app());
    command.env(
        "JCLI_OPEN_API_VERIFY_PATH",
        configuration::get_openapi_path(),
    );
    command
}

pub fn get_rest_settings_command(host: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app());
    command
        .arg("rest")
        .arg("v0")
        .arg("settings")
        .arg("get")
        .arg("--host")
        .arg(&host);
    command
}

pub fn get_stake_pools_command(host: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app());
    command
        .arg("rest")
        .arg("v0")
        .arg("stake-pools")
        .arg("get")
        .arg("--host")
        .arg(&host);
    command
}

pub fn get_stake_pool_command(stake_pool_id: &str, host: &str) -> Command {
    let mut command = Command::new(configuration::get_jcli_app());
    command
        .arg("rest")
        .arg("v0")
        .arg("stake-pool")
        .arg("get")
        .arg(&stake_pool_id)
        .arg("--host")
        .arg(&host);
    command
}
