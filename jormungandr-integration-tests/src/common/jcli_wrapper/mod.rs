#![allow(dead_code)]

use jormungandr_lib::interfaces::FragmentLog;

pub mod certificate;
pub mod jcli_commands;
pub mod jcli_transaction_wrapper;

use super::configuration;
use super::configuration::genesis_model::GenesisYaml;
use super::data::utxo::Utxo;
use super::file_assert;
use super::file_utils;
use super::process_assert;
use super::process_utils;
use super::process_utils::output_extensions::ProcessOutput;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(PartialEq)]
pub enum Discrimination {
    Production,
    Test,
}

pub fn assert_genesis_encode(
    genesis_yaml_file_path: &PathBuf,
    path_to_output_block: &PathBuf,
) -> () {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_genesis_encode_command(&genesis_yaml_file_path, &path_to_output_block),
    );
    process_assert::assert_process_exited_successfully(output);
    file_assert::assert_file_exists_and_not_empty(path_to_output_block);
}

pub fn assert_genesis_encode_fails(genesis_yaml: &GenesisYaml, expected_msg: &str) {
    let input_yaml_file_path = GenesisYaml::serialize(&genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");
    process_assert::assert_process_failed_and_matches_message(
        jcli_commands::get_genesis_encode_command(&input_yaml_file_path, &path_to_output_block),
        expected_msg,
    );
}

pub fn assert_genesis_hash(path_to_output_block: &PathBuf) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_genesis_hash_command(&path_to_output_block),
    );
    let hash = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    hash
}

pub fn assert_genesis_hash_fails(path_to_output_block: &PathBuf, expected_msg: &str) {
    process_assert::assert_process_failed_and_contains_message(
        jcli_commands::get_genesis_hash_command(&path_to_output_block),
        expected_msg,
    );
}

pub fn assert_rest_stats(host: &str) -> BTreeMap<String, String> {
    let output =
        process_utils::run_process_and_get_output(jcli_commands::get_rest_stats_command(&host));
    let content = output.as_single_node_yaml();
    process_assert::assert_process_exited_successfully(output);
    content
}

pub fn assert_rest_utxo_get(host: &str) -> Vec<Utxo> {
    let output =
        process_utils::run_process_and_get_output(jcli_commands::get_rest_utxo_get_command(&host));
    let content = output.as_lossy_string();
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

pub fn assert_get_address_info_fails(adress: &str, expected_msg: &str) {
    process_assert::assert_process_failed_and_contains_message(
        jcli_commands::get_address_info_command_default(&adress),
        expected_msg,
    );
}

pub fn assert_genesis_init() -> String {
    let output =
        process_utils::run_process_and_get_output(jcli_commands::get_genesis_init_command());
    let content = output.as_lossy_string();
    process_assert::assert_process_exited_successfully(output);
    content
}

pub fn assert_address_single(public_key: &str, discrimination: Discrimination) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_address_single_command(&public_key, discrimination),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_address_delegation(
    public_key: &str,
    delegation_key: &str,
    discrimination: Discrimination,
) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_address_delegation_command(&public_key, &delegation_key, discrimination),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_address_account(public_key: &str, discrimination: Discrimination) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_address_account_command(&public_key, discrimination),
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

pub fn assert_transaction_post_accepted(transaction_hash: &str, host: &str) -> () {
    let node_stats = self::assert_rest_stats(&host);
    let before: i32 = node_stats.get("txRecvCnt").unwrap().parse().unwrap();

    self::assert_post_transaction(&transaction_hash, &host);
    let node_stats = self::assert_rest_stats(&host);
    let after: i32 = node_stats.get("txRecvCnt").unwrap().parse().unwrap();
    assert_eq!(
        before + 1,
        after,
        "Transaction was NOT accepted by node:
     txRecvCnt counter wasn't incremented after post"
    );

    self::assert_rest_utxo_get(&host);
}

pub fn assert_transaction_post_failed(transaction_hash: &str, host: &str) -> () {
    let node_stats = self::assert_rest_stats(&host);
    let before: i32 = node_stats.get("txRecvCnt").unwrap().parse().unwrap();

    self::assert_post_transaction(&transaction_hash, &host);
    let node_stats = self::assert_rest_stats(&host);
    let after: i32 = node_stats.get("txRecvCnt").unwrap().parse().unwrap();
    assert_eq!(
        before, after,
        "Transaction was accepted by node while it should not be
     txRecvCnt counter was incremented after post"
    );

    self::assert_rest_utxo_get(&host);
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

pub fn assert_key_from_bytes_fails(
    path_to_input_file: &PathBuf,
    key_type: &str,
    expected_msg: &str,
) {
    process_assert::assert_process_failed_and_matches_message(
        jcli_commands::get_key_from_bytes_command(&path_to_input_file, &key_type),
        expected_msg,
    );
}

pub fn assert_key_to_bytes_fails(
    input_file: &PathBuf,
    path_to_output_file: &PathBuf,
    expected_msg: &str,
) {
    process_assert::assert_process_failed_and_matches_message(
        jcli_commands::get_key_to_bytes_command(&input_file, &path_to_output_file),
        expected_msg,
    );
}

pub fn assert_rest_get_block_tip(host: &str) -> String {
    let output =
        process_utils::run_process_and_get_output(jcli_commands::get_rest_block_tip_command(&host));
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_rest_account_get_stats(address: &str, host: &str) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_rest_account_stats_command(&address, &host),
    );
    let single_line = output.as_lossy_string();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_rest_get_block_by_id(block_id: &str, host: &str) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_rest_get_block_command(&block_id, &host),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_rest_get_next_block_id(block_id: &str, id_count: &i32, host: &str) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_rest_get_next_block_id_command(&block_id, &id_count, &host),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_rest_message_logs(host: &str) -> Vec<FragmentLog> {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_rest_message_log_command(&host),
    );
    let content = output.as_lossy_string();
    serde_yaml::from_str(&content).unwrap()
}

pub fn assert_transaction_in_block(transaction_message: &str, transaction_id: &str, host: &str) {
    self::assert_transaction_post_accepted(&transaction_message, &host);
    process_utils::run_process_until_response_matches(
        jcli_commands::get_rest_message_log_command(&host),
        |output| {
            let content = output.as_lossy_string();
            let fragments: Vec<FragmentLog> = serde_yaml::from_str(&content).unwrap();
            match fragments
                .iter()
                .find(|x| x.fragment_id().to_string() == transaction_id)
            {
                Some(x) => !x.is_pending(),
                None => false,
            }
        },
        1,
        5,
        &format!(
            "Waiting for transaction {} to be inBlock or rejected",
            &transaction_id
        ),
        "transaction is pending for too long",
    );
}
