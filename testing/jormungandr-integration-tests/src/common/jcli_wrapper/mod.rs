#![allow(dead_code)]

use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::{
    AccountState, FragmentLog, FragmentStatus, LeadershipLog, SettingsDto, StakePoolStats,
    UTxOInfo, UTxOOutputInfo, CommitteeIdDef
};
pub mod certificate;
pub mod jcli_commands;
pub mod jcli_transaction_wrapper;

pub use jcli_transaction_wrapper::JCLITransactionWrapper;

use super::configuration;
use super::process_assert;
use super::process_utils::{self, output_extensions::ProcessOutput, Wait};
use crate::common::jormungandr::JormungandrProcess;
use chain_addr::Discrimination;

use assert_fs::prelude::*;
use assert_fs::{fixture::ChildPath, NamedTempFile};
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;
use serde_json::Value;

#[derive(Debug, Error)]
pub enum Error {
    #[error("transaction {transaction_id} is not in block. message log: {message_log}. Jormungandr log: {log_content}")]
    TransactionNotInBlock {
        message_log: String,
        transaction_id: Hash,
        log_content: String,
    },
    #[error("at least one transaction is not in block. message log: {message_log}. Jormungandr log: {log_content}")]
    TransactionsNotInBlock {
        message_log: String,
        log_content: String,
    },
}

pub fn assert_genesis_encode(genesis_yaml_file_path: &Path, output_file: &ChildPath) {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_genesis_encode_command(genesis_yaml_file_path, output_file.path()),
    );
    process_assert::assert_process_exited_successfully(output);
    output_file.assert(crate::predicate::file_exists_and_not_empty());
}

pub fn assert_genesis_decode(genesis_yaml_file_path: &Path, output_file: &ChildPath) {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_genesis_decode_command(genesis_yaml_file_path, output_file.path()),
    );
    process_assert::assert_process_exited_successfully(output);
    output_file.assert(crate::predicate::file_exists_and_not_empty());
}

pub fn assert_genesis_encode_fails(
    genesis_yaml_file_path: &Path,
    output_file: &ChildPath,
    expected_msg: &str,
) {
    process_assert::assert_process_failed_and_matches_message(
        jcli_commands::get_genesis_encode_command(genesis_yaml_file_path, output_file.path()),
        expected_msg,
    );
}

pub fn assert_genesis_hash(path_to_output_block: &Path) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_genesis_hash_command(&path_to_output_block),
    );
    let hash = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    hash
}

pub fn assert_genesis_hash_fails(path_to_output_block: &Path, expected_msg: &str) {
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

pub fn assert_rest_utxo_get_returns_same_utxo(host: &str, utxo: &UTxOInfo) {
    let rest_utxo = assert_rest_utxo_get_by_utxo(host, utxo);
    assert_eq!(utxo, &rest_utxo, "UTxO returned from REST is invalid");
}

pub fn assert_rest_utxo_get_by_utxo(host: &str, utxo: &UTxOInfo) -> UTxOInfo {
    assert_rest_utxo_get(
        host,
        &utxo.transaction_id().to_string(),
        utxo.index_in_transaction(),
    )
}

pub fn assert_rest_utxo_get(host: &str, fragment_id_bech32: &str, output_index: u8) -> UTxOInfo {
    let command = jcli_commands::get_rest_utxo_get_command(&host, fragment_id_bech32, output_index);
    let output = process_utils::run_process_and_get_output(command);
    let content = output.as_lossy_string();
    process_assert::assert_process_exited_successfully(output);
    let fragment_id = fragment_id_bech32
        .parse()
        .expect("UTxO fragment ID is not a valid hex value");
    serde_yaml::from_str::<UTxOOutputInfo>(&content)
        .expect("JCLI returned malformed UTxO")
        .into_utxo_info(fragment_id, output_index)
}

pub fn assert_rest_utxo_get_by_utxo_not_found(host: &str, utxo: &UTxOInfo) {
    assert_rest_utxo_get_not_found(
        host,
        &utxo.transaction_id().to_string(),
        utxo.index_in_transaction(),
    )
}

pub fn assert_rest_utxo_get_not_found(host: &str, fragment_id_bech32: &str, output_index: u8) {
    let command = jcli_commands::get_rest_utxo_get_command(&host, fragment_id_bech32, output_index);
    process_assert::assert_process_failed_and_contains_message(command, "404 Not Found");
}

pub fn assert_get_address_info(address: &str) -> BTreeMap<String, String> {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_address_info_command_default(&address),
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

pub fn assert_post_transaction(transactions_message: &str, host: &str) -> Hash {
    let transaction_file = NamedTempFile::new("transaction.hash").unwrap();
    transaction_file.write_str(transactions_message).unwrap();
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_post_transaction_command(transaction_file.path(), host),
    );
    let hash = output.as_hash();
    process_assert::assert_process_exited_successfully(output);
    hash
}

pub fn assert_transaction_post_accepted(transactions_message: &str, host: &str) {
    let node_stats = assert_rest_stats(&host);
    let before: i32 = node_stats.get("txRecvCnt").unwrap().parse().unwrap();

    assert_post_transaction(&transactions_message, &host);
    let node_stats = assert_rest_stats(&host);
    let after: i32 = node_stats.get("txRecvCnt").unwrap().parse().unwrap();
    assert_eq!(
        before + 1,
        after,
        "Transaction was NOT accepted by node: \
        txRecvCnt counter wasn't incremented after post"
    );
}

pub fn assert_transaction_post_failed(transactions_message: &str, host: &str) {
    let node_stats = assert_rest_stats(&host);
    let before: i32 = node_stats.get("txRecvCnt").unwrap().parse().unwrap();

    assert_post_transaction(&transactions_message, &host);
    let node_stats = assert_rest_stats(&host);
    let after: i32 = node_stats.get("txRecvCnt").unwrap().parse().unwrap();
    assert_eq!(
        before, after,
        "Transaction was accepted by node while it should not be: \
        txRecvCnt counter was incremented after post"
    );
}

pub fn assert_get_active_voting_committees(host: &str) -> Vec<CommitteeIdDef> {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_rest_active_committes(host),
    );
    let content = output.as_lossy_string();
    process_assert::assert_process_exited_successfully(output.clone());
    serde_yaml::from_str(&content)
        .expect("JCLI returned malformed CommitteeIdDef")  
}

pub fn assert_get_active_vote_plans(host: &str) -> Vec<Value> {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_rest_active_vote_plans(host),
    );
    let content = output.as_lossy_string();
    process_assert::assert_process_exited_successfully(output.clone());
    serde_yaml::from_str(&content)
        .expect("JCLI returned malformed VotePlan")  
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
    let input_file = NamedTempFile::new("key_to_public.input").unwrap();
    input_file.write_str(private_key).unwrap();
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_key_to_public_command(input_file.path()),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_key_to_public_fails(private_key: &str, expected_msg: &str) {
    let input_file = NamedTempFile::new("key_to_public.input").unwrap();
    input_file.write_str(private_key).unwrap();
    process_assert::assert_process_failed_and_contains_message(
        jcli_commands::get_key_to_public_command(input_file.path()),
        expected_msg,
    );
}

pub fn assert_key_to_bytes(private_key: &str, path_to_output_file: &Path) {
    let input_file = NamedTempFile::new("key_to_bytes.input").unwrap();
    input_file.write_str(private_key).unwrap();

    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_key_to_bytes_command(input_file.path(), &path_to_output_file),
    );
    process_assert::assert_process_exited_successfully(output);
}

pub fn assert_key_from_bytes(path_to_input_file: &Path, key_type: &str) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_key_from_bytes_command(&path_to_input_file, &key_type),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_key_from_bytes_fails(path_to_input_file: &Path, key_type: &str, expected_msg: &str) {
    process_assert::assert_process_failed_and_contains_message(
        jcli_commands::get_key_from_bytes_command(&path_to_input_file, &key_type),
        expected_msg,
    );
}

pub fn assert_key_to_bytes_fails(
    input_file: &Path,
    path_to_output_file: &Path,
    expected_msg: &str,
) {
    process_assert::assert_process_failed_and_matches_message(
        jcli_commands::get_key_to_bytes_command(&input_file, &path_to_output_file),
        expected_msg,
    );
}

pub fn assert_rest_get_leadership_log(host: &str) -> Vec<LeadershipLog> {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_rest_leaders_logs_command(&host),
    );
    let content = output.as_lossy_string();
    process_assert::assert_process_exited_successfully(output);

    serde_yaml::from_str(&content).unwrap()
}

pub fn assert_rest_get_block_tip(host: &str) -> String {
    let output =
        process_utils::run_process_and_get_output(jcli_commands::get_rest_block_tip_command(&host));
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_rest_account_get_stats(address: &str, host: &str) -> AccountState {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_rest_account_stats_command(&address, &host),
    );
    let content = output.as_lossy_string();
    process_assert::assert_process_exited_successfully(output);

    serde_yaml::from_str(&content).unwrap()
}

pub fn assert_rest_shutdown(host: &str) {
    let output =
        process_utils::run_process_and_get_output(jcli_commands::get_rest_shutdown_command(&host));
    process_assert::assert_process_exited_successfully(output);
}

pub fn assert_rest_get_block_by_id(block_id: &str, host: &str) -> String {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_rest_get_block_command(&block_id, &host),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    single_line
}

pub fn assert_rest_get_next_block_id(block_id: &str, id_count: i32, host: &str) -> Hash {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_rest_get_next_block_id_command(&block_id, id_count, &host),
    );
    let single_line = output.as_single_line();
    process_assert::assert_process_exited_successfully(output);
    Hash::from_str(&single_line).unwrap()
}

pub fn assert_transaction_in_block(
    transaction_message: &str,
    jormungandr: &JormungandrProcess,
) -> Hash {
    let fragment_id = assert_post_transaction(&transaction_message, &jormungandr.rest_uri());
    let wait: Wait = Default::default();
    wait_until_transaction_processed(fragment_id, jormungandr, &wait).unwrap();
    assert_transaction_log_shows_in_block(fragment_id, jormungandr);
    fragment_id
}

pub fn assert_transaction_in_block_with_wait(
    transaction_message: &str,
    jormungandr: &JormungandrProcess,
    wait: &Wait,
) -> Hash {
    let fragment_id = assert_post_transaction(&transaction_message, &jormungandr.rest_uri());
    wait_until_transaction_processed(fragment_id, jormungandr, wait).unwrap();
    assert_transaction_log_shows_in_block(fragment_id, jormungandr);
    fragment_id
}

pub fn assert_transaction_rejected(
    transaction_message: &str,
    jormungandr: &JormungandrProcess,
    expected_reason: &str,
) {
    let fragment_id = assert_post_transaction(&transaction_message, &jormungandr.rest_uri());
    let wait: Wait = Default::default();
    wait_until_transaction_processed(fragment_id, jormungandr, &wait).unwrap();
    assert_transaction_log_shows_rejected(fragment_id, jormungandr, &expected_reason);
}

pub fn wait_until_transaction_processed(
    fragment_id: Hash,
    jormungandr: &JormungandrProcess,
    wait: &Wait,
) -> Result<(), Error> {
    process_utils::run_process_until_response_matches(
        jcli_commands::get_rest_message_log_command(&jormungandr.rest_uri()),
        |output| {
            let content = output.as_lossy_string();
            let fragments: Vec<FragmentLog> =
                serde_yaml::from_str(&content).expect("Cannot parse fragment logs");
            match fragments.iter().find(|x| *x.fragment_id() == fragment_id) {
                Some(x) => {
                    println!("Transaction found in mempool. {:?}", x);
                    !x.is_pending()
                }
                None => {
                    println!("Transaction with hash {} not found in mempool", fragment_id);
                    false
                }
            }
        },
        wait.sleep_duration().as_secs(),
        wait.attempts(),
        &format!(
            "Waiting for transaction: '{}' to be inBlock or rejected",
            fragment_id
        ),
        &format!(
            "transaction: '{}' is pending for too long, Logs: {:?}",
            fragment_id,
            jormungandr.logger.get_log_content()
        ),
    )
    .map_err(|_| Error::TransactionNotInBlock {
        message_log: format!("{:?}", assert_get_rest_message_log(&jormungandr.rest_uri())),
        transaction_id: fragment_id,
        log_content: jormungandr.logger.get_log_content(),
    })
}

pub fn assert_transaction_log_shows_in_block(fragment_id: Hash, jormungandr: &JormungandrProcess) {
    let fragments = assert_get_rest_message_log(&jormungandr.rest_uri());
    match fragments.iter().find(|x| *x.fragment_id() == fragment_id) {
        Some(x) => assert!(
            x.is_in_a_block(),
            "Fragment should be in block, actual: {:?}. Logs: {:?}",
            &x,
            jormungandr.logger.get_log_content()
        ),
        None => panic!(
            "cannot find any fragment in rest message log, output: {:?}. Node log: {:?}",
            &fragments,
            jormungandr.logger.get_log_content()
        ),
    }
}

pub fn assert_transaction_log_shows_rejected(
    fragment_id: Hash,
    jormungandr: &JormungandrProcess,
    expected_msg: &str,
) {
    let fragments = assert_get_rest_message_log(&jormungandr.rest_uri());
    match fragments.iter().find(|x| *x.fragment_id() == fragment_id) {
        Some(x) => {
            assert!(
                x.is_rejected(),
                "Fragment should be rejected, actual: {:?}. Logs: {:?}",
                &x,
                jormungandr.logger.get_log_content()
            );
            match x.status() {
                FragmentStatus::Rejected { reason } => assert!(reason.contains(&expected_msg)),
                _ => panic!("Non expected state for for rejected log"),
            }
        }
        None => panic!(
            "cannot find any fragment in rest message log, output: {:?}. Logs: {:?}",
            &fragments,
            jormungandr.logger.get_log_content()
        ),
    }
}

pub fn send_transactions_and_wait_until_in_block(
    transactions_messages: &[String],
    jormungandr: &JormungandrProcess,
) -> Result<(), Error> {
    for transactions_message in transactions_messages.iter() {
        assert_post_transaction(&transactions_message, &jormungandr.rest_uri());
    }
    wait_until_all_transactions_processed(&jormungandr)?;
    check_all_transaction_log_shows_in_block(&jormungandr)
}

pub fn wait_until_all_transactions_processed(
    jormungandr: &JormungandrProcess,
) -> Result<(), Error> {
    process_utils::run_process_until_response_matches(
        jcli_commands::get_rest_message_log_command(&jormungandr.rest_uri()),
        |output| {
            let content = output.as_lossy_string();
            let fragments: Vec<FragmentLog> =
                serde_yaml::from_str(&content).expect("Cannot parse fragment logs");
            let at_least_one_pending = fragments.iter().any(|x| x.is_pending());
            !at_least_one_pending
        },
        1,
        5,
        "Waiting for last transaction to be inBlock or rejected",
        "transaction is pending for too long",
    )
    .map_err(|_| Error::TransactionsNotInBlock {
        message_log: format!("{:?}", assert_get_rest_message_log(&jormungandr.rest_uri())),
        log_content: jormungandr.logger.get_log_content(),
    })
}

pub fn check_all_transaction_log_shows_in_block(
    jormungandr: &JormungandrProcess,
) -> Result<(), Error> {
    let fragments = assert_get_rest_message_log(&jormungandr.rest_uri());
    for fragment in fragments.iter() {
        if !fragment.is_in_a_block() {
            return Err(Error::TransactionNotInBlock {
                message_log: format!("{:?}", fragments.clone()),
                transaction_id: *fragment.fragment_id(),
                log_content: jormungandr.logger.get_log_content(),
            });
        }
    }
    Ok(())
}

pub fn assert_get_rest_message_log(host: &str) -> Vec<FragmentLog> {
    let output = process_utils::run_process_and_get_output(
        jcli_commands::get_rest_message_log_command(&host),
    );
    let content = output.as_lossy_string();
    process_assert::assert_process_exited_successfully(output);
    let fragments: Vec<FragmentLog> =
        serde_yaml::from_str(&content).expect("Failed to parse fragment log");
    fragments
}

pub fn assert_get_rest_settings(host: &str) -> SettingsDto {
    let output =
        process_utils::run_process_and_get_output(jcli_commands::get_rest_settings_command(&host));
    let content = output.as_lossy_string();
    process_assert::assert_process_exited_successfully(output);
    let settings: SettingsDto = serde_yaml::from_str(&content).expect("Failed to parse settings");
    settings
}

pub fn assert_rest_get_stake_pools(host: &str) -> Vec<String> {
    let output =
        process_utils::run_process_and_get_output(jcli_commands::get_stake_pools_command(&host));
    let content = output.as_lossy_string();
    process_assert::assert_process_exited_successfully(output);
    serde_yaml::from_str(&content).expect("Failed to parse stake poools collection")
}

pub fn assert_rest_get_stake_pool(stake_pool_id: &str, host: &str) -> StakePoolStats {
    let output = process_utils::run_process_and_get_output(jcli_commands::get_stake_pool_command(
        &stake_pool_id,
        &host,
    ));
    let content = output.as_lossy_string();
    process_assert::assert_process_exited_successfully(output);
    serde_yaml::from_str(&content).expect("Failed to parse stak pool stats")
}
