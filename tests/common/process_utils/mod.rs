extern crate serde_yaml;

pub mod process_guard;
use super::process_assert;
use std::collections::BTreeMap;
use std::process::{Command, Stdio};
use std::{thread, time};

/// Runs command, wait for output and returns it as a single yaml node
///
/// # Arguments
///
/// * `command` - Command which will be invoked
///
pub fn run_process_and_get_yaml_single(command: Command) -> BTreeMap<String, String> {
    let content = run_process_and_get_output(command);
    let deserialized_map: BTreeMap<String, String> = serde_yaml::from_str(&content).unwrap();
    deserialized_map
}

/// Runs command, wait for output and returns it as a collection of yaml nodes
///
/// # Arguments
///
/// * `command` - Command which will be invoked
///
pub fn run_process_and_get_yaml_collection(command: Command) -> Vec<BTreeMap<String, String>> {
    let content = run_process_and_get_output(command);
    let deserialized_map: Vec<BTreeMap<String, String>> = serde_yaml::from_str(&content).unwrap();
    deserialized_map
}

/// Runs command, wait for output and returns it as a string
///
/// # Arguments
///
/// * `command` - Command which will be invoked
///
pub fn run_process_and_get_output(mut command: Command) -> String {
    let content = command
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .expect("failed to execute process");

    process_assert::assert_process_exited_successfully(content.clone());

    let content = String::from_utf8_lossy(&content.stdout).into_owned();
    content
}

pub fn run_process_and_get_output_line(command: Command) -> String {
    let mut content = run_process_and_get_output(command);
    if content.ends_with("\n") {
        let len = content.len();
        content.truncate(len - 1);
    }
    content
}

/// Runs command for n times with m second interval.
///
/// # Panics
///
/// Panics if max_attempts is exceeded and none of attempts return successful exit code.
///
/// # Arguments
///
/// * `command` - Command which will be invoked
/// * `timeout` - Timeout after unsuccesful attempt (in seconds)
/// * `max_attempts` - Maximum number of attempts
/// * `command_description` - User description of command
/// * `error_description` - User description of error
///
/// # Example
///
/// use process_utils::run_process_until_exited_successfully;
///
///    process_utils::run_process_until_exited_successfully(
///         jcli_wrapper::run_rest_stats_command_default(),
///         2,
///         5,
///         "get stats from jormungandr node",
///         "jormungandr node is not up"
///    );
///
pub fn run_process_until_exited_successfully(
    mut command: Command,
    timeout: u64,
    max_attempts: i32,
    command_description: &str,
    error_description: &str,
) {
    let one_second = time::Duration::from_millis(&timeout * 1000);
    let mut attempts = max_attempts.clone();

    loop {
        if command
            .status()
            .expect(&format!(
                "failed to get exit status of command: {}",
                &command_description
            ))
            .success()
        {
            break;
        }

        if attempts <= 0 {
            break;
        }

        println!(
            "non-zero status with message(). waiting {} s and trying again ({} of {})",
            &timeout,
            &max_attempts - &attempts + 1,
            &max_attempts
        );

        attempts = attempts - 1;
        thread::sleep(one_second);
    }

    if attempts <= 0 {
        panic!(
            "{} (tried to connect {} times with {} s interval)",
            &error_description, &max_attempts, &timeout
        );
    }
    println!("Success: {}", &command_description);
}
