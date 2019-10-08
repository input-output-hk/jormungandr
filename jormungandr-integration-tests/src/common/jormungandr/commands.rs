use crate::common::configuration;

use std::fs::File;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn set_json_logger(command: &mut Command) {
    command.arg("--log-format").arg("json");
}

pub fn get_start_jormungandr_node_command(
    config_path: &PathBuf,
    genesis_block_path: &PathBuf,
    log_file_path: &PathBuf,
) -> Command {
    let mut command = Command::new(configuration::get_jormungandr_app().as_os_str());
    set_json_logger(&mut command);
    command
        .arg("--config")
        .arg(config_path.as_os_str())
        .arg("--genesis-block")
        .arg(genesis_block_path.as_os_str())
        .stderr(get_stdio_from_log_file(&log_file_path));
    println!("Running start jormungandr command: {:?}", &command);
    command
}

pub fn get_start_jormungandr_as_leader_node_command(
    config_path: &PathBuf,
    genesis_block_path: &PathBuf,
    secret_path: &PathBuf,
    log_file_path: &PathBuf,
) -> Command {
    let mut command = Command::new(configuration::get_jormungandr_app().as_os_str());
    set_json_logger(&mut command);
    command
        .arg("--secret")
        .arg(secret_path.as_os_str())
        .arg("--config")
        .arg(config_path.as_os_str())
        .arg("--genesis-block")
        .arg(genesis_block_path.as_os_str())
        .stderr(get_stdio_from_log_file(&log_file_path));
    println!("Running start jormungandr command: {:?}", &command);
    command
}

pub fn get_start_jormungandr_as_slave_node_command(
    config_path: &PathBuf,
    genesis_block_hash: &str,
    log_file_path: &PathBuf,
) -> Command {
    let mut command = Command::new(configuration::get_jormungandr_app().as_os_str());
    set_json_logger(&mut command);
    command
        .arg("--config")
        .arg(config_path.as_os_str())
        .arg("--genesis-block-hash")
        .arg(&genesis_block_hash)
        .stderr(get_stdio_from_log_file(&log_file_path));
    println!("Running start jormungandr command: {:?}", &command);
    command
}

pub fn get_start_jormungandr_as_passive_node_command(
    config_path: &PathBuf,
    genesis_block_hash: &String,
    log_file_path: &PathBuf,
) -> Command {
    let mut command = Command::new(configuration::get_jormungandr_app().as_os_str());
    set_json_logger(&mut command);
    command
        .arg("--config")
        .arg(config_path.as_os_str())
        .arg("--genesis-block-hash")
        .arg(&genesis_block_hash)
        .stderr(get_stdio_from_log_file(&log_file_path));
    println!("Running start jormungandr command: {:?}", &command);
    command
}

#[cfg(windows)]
fn get_stdio_from_log_file(log_file_path: &PathBuf) -> std::process::Stdio {
    use std::os::windows::io::{FromRawHandle, IntoRawHandle};
    let file = File::create(log_file_path).expect("couldn't create log file for jormungandr");
    unsafe { Stdio::from_raw_handle(file.into_raw_handle()) }
}

#[cfg(unix)]
fn get_stdio_from_log_file(log_file_path: &PathBuf) -> std::process::Stdio {
    use std::os::unix::io::{FromRawFd, IntoRawFd};
    let file = File::create(log_file_path).expect("couldn't create log file for jormungandr");
    unsafe { Stdio::from_raw_fd(file.into_raw_fd()) }
}
