#![allow(dead_code)]

extern crate serde_yaml;

pub mod output_extensions;
mod wait;

pub use wait::{Wait, WaitBuilder};

use self::output_extensions::ProcessOutput;
use std::{
    process::{Command, Output, Stdio},
    thread, time,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("could not start process '{message}'")]
    ProcessExited { message: String },
}

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

    if content.as_lossy_string() != "" {
        println!("Output: {}", content.as_lossy_string());
    }
    if content.err_as_lossy_string() != "" {
        println!("Error: {}", content.err_as_lossy_string());
    }
    println!();
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
            "non-zero status with message(). {}. waiting {} s and trying again ({} of {})",
            command_description,
            &timeout,
            &max_attempts - &attempts + 1,
            &max_attempts
        );

        attempts = attempts - 1;
        self::sleep(timeout);
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
    sleep_between_attempt: u64,
    max_attempts: u64,
    command_description: &str,
    error_description: &str,
) -> Result<(), ProcessError> {
    let sleep_between_attempt_duration = time::Duration::from_millis(&sleep_between_attempt * 1000);
    let mut attempts = 1;

    println!("Running command {:?} in loop", command);

    loop {
        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
            .wait_with_output()
            .expect(&format!("cannot get output from command {:?}", &command));

        println!("Standard Output: {}", output.as_lossy_string());
        println!("Standard Error: {}", output.err_as_lossy_string());

        if output.status.success() && is_output_ok(output) {
            println!("Success: {}", &command_description);
            return Ok(());
        }

        if attempts >= max_attempts {
            return Err(ProcessError::ProcessExited {
                message: format!(
                    "{} (tried to connect {} times with {} s interval)",
                    &error_description, &max_attempts, &sleep_between_attempt
                ),
            });
        }

        println!(
            "non-zero status with message(). waiting {} s and trying again ({} of {})",
            &sleep_between_attempt, &attempts, &max_attempts
        );

        attempts = attempts + 1;
        thread::sleep(sleep_between_attempt_duration);
    }
}

pub fn sleep(seconds: u64) {
    let duration = time::Duration::from_secs(seconds);
    thread::sleep(duration);
}
