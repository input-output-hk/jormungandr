#![allow(dead_code)]

use super::configuration;
use std::path::PathBuf;
use std::process::Command;

pub fn get_start_jormungandr_node_command(
    config_path: &PathBuf,
    genesis_block_path: &PathBuf,
) -> Command {
    let mut command = Command::new(configuration::get_jormungandr_app().as_os_str());
    command
        .arg("--config")
        .arg(config_path.as_os_str())
        .arg("--genesis-block")
        .arg(genesis_block_path.as_os_str());
    println!("Running start jormungandr command: {:?}", &command);
    command
}

pub fn get_start_jormungandr_as_leader_node_command(
    config_path: &PathBuf,
    genesis_block_path: &PathBuf,
    secret_path: &PathBuf,
) -> Command {
    let mut command = Command::new(configuration::get_jormungandr_app().as_os_str());
    command
        .arg("--secret")
        .arg(secret_path.as_os_str())
        .arg("--config")
        .arg(config_path.as_os_str())
        .arg("--genesis-block")
        .arg(genesis_block_path.as_os_str());
    println!("Running start jormungandr command: {:?}", &command);
    command
}

pub fn get_start_jormungandr_as_slave_node_command(
    config_path: &PathBuf,
    genesis_block_hash: &str,
) -> Command {
    let mut command = Command::new(configuration::get_jormungandr_app().as_os_str());

    command
        .arg("--config")
        .arg(config_path.as_os_str())
        .arg("--genesis-block-hash")
        .arg(&genesis_block_hash)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    println!("Running start jormungandr command: {:?}", &command);

    command
}
