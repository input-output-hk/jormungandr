extern crate custom_error;

use self::custom_error::custom_error;
use crate::common::configuration::jormungandr_config::JormungandrConfig;
use crate::common::file_utils;
use crate::common::jcli_wrapper;
use crate::common::jormungandr::{commands, process::JormungandrProcess};

use crate::common::process_assert;
use crate::common::process_utils::{self, output_extensions::ProcessOutput, ProcessError};
use std::process::{Child, Command, Output};

custom_error! {pub StartupError
    JormungandrNotLaunched{ source: ProcessError } = "could not start jormungandr",
}

fn try_to_start_jormungandr_node(
    command: &mut Command,
    config: JormungandrConfig,
) -> Result<Child, StartupError> {
    println!("Starting jormungandr node...");
    let process = command
        .spawn()
        .expect("failed to execute 'start jormungandr node'");

    let proces_start_result = process_utils::run_process_until_response_matches(
        jcli_wrapper::jcli_commands::get_rest_stats_command(&config.get_node_address()),
        &is_node_up,
        2,
        5,
        "get stats from jormungandr node",
        "jormungandr node is not up",
    );

    match proces_start_result {
        Ok(_) => return Ok(process),
        Err(e) => return Err(StartupError::JormungandrNotLaunched { source: e }),
    }
}

fn start_jormungandr_node_sync_with_retry(
    command: &mut Command,
    config: &mut JormungandrConfig,
) -> JormungandrProcess {
    let first_attempt = try_to_start_jormungandr_node(command, config.clone());
    match first_attempt {
        Ok(guard) => return JormungandrProcess::from_config(guard, config.clone()),
        _ => println!("failed to start jormungandr node. retrying.."),
    };
    config.refresh_node_dynamic_params();
    let second_attempt = try_to_start_jormungandr_node(command, config.clone());

    match second_attempt {
        Ok(guard) => return JormungandrProcess::from_config(guard, config.clone()),
        Err(e) => {
            let log_file_content = file_utils::read_file(&config.log_file_path);
            panic!(format!("{}. Log file: {}", e.to_string(), log_file_content));
        }
    };
}

fn is_node_up(output: Output) -> bool {
    match output.as_single_node_yaml().get("uptime") {
        Some(uptime) => {
            return uptime
                .parse::<i32>()
                .expect(&format!("Cannot parse uptime {}", uptime.to_string()))
                > 2
        }
        None => return false,
    }
}

pub fn start_jormungandr_node(config: &mut JormungandrConfig) -> JormungandrProcess {
    let mut command = commands::get_start_jormungandr_node_command(
        &config.node_config_path,
        &config.genesis_block_path,
        &config.log_file_path,
    );

    println!("Starting node with configuration : {:?}", &config);
    let process = start_jormungandr_node_sync_with_retry(&mut command, config);
    process
}

pub fn restart_jormungandr_node_as_leader(process: &mut JormungandrProcess) -> JormungandrProcess {
    let mut config = process.config.clone();
    config.refresh_node_dynamic_params();
    std::mem::drop(process);

    println!("Starting node with configuration : {:?}", &config);

    let mut command = commands::get_start_jormungandr_as_leader_node_command(
        &config.node_config_path,
        &config.genesis_block_path,
        &config.secret_model_path,
        &config.log_file_path,
    );

    match try_to_start_jormungandr_node(&mut command, config.clone()) {
        Ok(guard) => return JormungandrProcess::from_config(guard, config.clone()),
        Err(e) => {
            let log_file_content = file_utils::read_file(&config.log_file_path);
            panic!(format!("{}. Log file: {}", e.to_string(), log_file_content));
        }
    };
}

pub fn start_jormungandr_node_as_leader(config: &mut JormungandrConfig) -> JormungandrProcess {
    let mut command = commands::get_start_jormungandr_as_leader_node_command(
        &config.node_config_path,
        &config.genesis_block_path,
        &config.secret_model_path,
        &config.log_file_path,
    );
    println!("Starting node with configuration : {:?}", &config);
    let process = start_jormungandr_node_sync_with_retry(&mut command, config);
    process
}

pub fn start_jormungandr_node_as_slave(config: &mut JormungandrConfig) -> JormungandrProcess {
    let mut command = commands::get_start_jormungandr_as_slave_node_command(
        &config.node_config_path,
        &config.genesis_block_hash,
        &config.log_file_path,
    );

    println!("Starting node with configuration : {:?}", &config);
    let process = start_jormungandr_node_sync_with_retry(&mut command, config);
    process
}

pub fn start_jormungandr_node_as_passive(config: &mut JormungandrConfig) -> JormungandrProcess {
    let mut command = commands::get_start_jormungandr_as_passive_node_command(
        &config.node_config_path,
        &config.genesis_block_hash,
        &config.secret_model_path,
        &config.log_file_path,
    );
    let process = start_jormungandr_node_sync_with_retry(&mut command, config);
    process
}

pub fn assert_start_jormungandr_node_as_passive_fail(
    config: &mut JormungandrConfig,
    expected_msg: &str,
) {
    let command = commands::get_start_jormungandr_as_passive_node_command(
        &config.node_config_path,
        &config.genesis_block_hash,
        &config.secret_model_path,
        &config.log_file_path,
    );

    process_assert::assert_process_failed_and_matches_message(command, &expected_msg);
}
