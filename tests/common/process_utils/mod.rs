extern crate serde_yaml;

pub mod output_extensions;
pub mod process_guard;
use self::output_extensions::ProcessOutput;
use std::process::{Command, Output, Stdio};
use std::{thread, time};

/// Runs command, wait for output and returns it output
///
/// # Arguments
///
/// * `command` - Command which will be invoked
///
pub fn run_process_and_get_output(mut command: Command) -> Output {
    println!("Running command: {:?}", &command);
    let content = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .expect("failed to execute process");

    println!("Standard Output: {}", content.as_lossy_string());
    println!("Standard Error: {}", content.err_as_lossy_string());
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

pub fn run_process_until_response_matches<F: Fn(Output) -> bool>(
    mut command: Command,
    is_output_ok: F,
    timeout: u64,
    max_attempts: i32,
    command_description: &str,
    error_description: &str,
) {
    let one_second = time::Duration::from_millis(&timeout * 1000);
    let mut attempts = max_attempts.clone();

    loop {
        let output = command
            .output()
            .expect(&format!("cannot get output from command {:?}", &command));
        if is_output_ok(output) {
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
