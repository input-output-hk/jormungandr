#![cfg(feature = "integration-test")]

extern crate custom_error;

use self::custom_error::custom_error;

use common::configuration::jormungandr_config::JormungandrConfig;
use common::jcli_wrapper;
use common::jormungandr_wrapper;
use common::process_utils;
use common::process_utils::{
    output_extensions::ProcessOutput, process_guard::ProcessKillGuard, ProcessError,
};

use std::process::{Command, Output};

custom_error! {pub StartupError
    JormungandrNotLaunched{ source: ProcessError } = "could not start jormungandr",
}

fn try_to_start_jormungandr_node(
    rest_address: &str,
    command: &mut Command,
) -> Result<ProcessKillGuard, StartupError> {
    println!("Starting jormungandr node...");
    let process = command
        .spawn()
        .expect("failed to execute 'start jormungandr node'");

    let guard = ProcessKillGuard::new(process, String::from("Jormungandr node"));

    let proces_start_result = process_utils::run_process_until_response_matches(
        jcli_wrapper::jcli_commands::get_rest_stats_command(&rest_address),
        &is_node_up,
        2,
        5,
        "get stats from jormungandr node",
        "jormungandr node is not up",
    );

    match proces_start_result {
        Ok(_) => return Ok(guard),
        Err(e) => return Err(StartupError::JormungandrNotLaunched { source: e }),
    }
}

fn start_jormungandr_node_sync_with_retry(
    rest_address: &str,
    command: &mut Command,
    config: &mut JormungandrConfig,
) -> ProcessKillGuard {
    let first_attempt = try_to_start_jormungandr_node(rest_address, command);
    match first_attempt {
        Ok(guard) => return guard,
        _ => println!("failed to start jormungandr node. retrying.."),
    };
    config.node_config.regenerate_ports();
    let second_attempt = try_to_start_jormungandr_node(rest_address, command);
    match second_attempt {
        Ok(guard) => return guard,
        Err(e) => panic!(e.to_string()),
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

pub fn start_jormungandr_node(mut config: &mut JormungandrConfig) -> ProcessKillGuard {
    let rest_address = &config.node_config.get_node_address();

    let mut command = jormungandr_wrapper::get_start_jormungandr_node_command(
        &config.node_config_path,
        &config.genesis_block_path,
    );

    println!("Starting node with configuration : {:?}", &config);
    let process = start_jormungandr_node_sync_with_retry(&rest_address, &mut command, &mut config);
    process
}

pub fn start_jormungandr_node_as_leader(mut config: &mut JormungandrConfig) -> ProcessKillGuard {
    let rest_address = &config.node_config.get_node_address();

    let mut command = jormungandr_wrapper::get_start_jormungandr_as_leader_node_command(
        &config.node_config_path,
        &config.genesis_block_path,
        &config.secret_model_path,
    );

    println!("Starting node with configuration : {:?}", &config);
    let process = start_jormungandr_node_sync_with_retry(&rest_address, &mut command, &mut config);
    process
}

pub fn start_jormungandr_node_as_slave(mut config: &mut JormungandrConfig) -> ProcessKillGuard {
    let rest_address = &config.node_config.get_node_address();

    let mut command = jormungandr_wrapper::get_start_jormungandr_as_slave_node_command(
        &config.node_config_path,
        &config.genesis_block_hash,
    );

    println!("Starting node with configuration : {:?}", &config);
    let process = start_jormungandr_node_sync_with_retry(&rest_address, &mut command, &mut config);
    process
}
