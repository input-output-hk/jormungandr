#![allow(dead_code)]

extern crate regex;

use self::regex::Regex;
use std::process::{Command, Output};

use super::process_utils;
use super::process_utils::output_extensions::ProcessOutput;

/// Assert process exited successfully
///
/// # Arguments
///
/// * `command` - Command which will be invoked
/// * `description` - User description of command
///
/// # Example
///
/// use process_assert::assert_process_exited_successfully;
///
/// let command = Command::new("mkdir");
/// let description = "mkdir command";
/// assert_process_exited_successfully(&command,&description);
///
pub fn run_and_assert_process_exited_successfully(mut command: Command, description: &str) {
    let mut process = command
        .spawn()
        .expect(&format!("failed to execute {} command", &description));

    let exit_code = process
        .wait()
        .expect(&format!("failed to wait for {} to finish", &description));

    assert!(
        exit_code.success(),
        "non-zero exit code {} of command {}",
        &exit_code.code().unwrap(),
        &description
    );
}

/// Asserts process has non-zero exit code and finished with an error
pub fn assert_process_failed(output: Output) {
    println!("Running transaction new command...");

    assert_eq!(
        output.status.success(),
        false,
        "Unexpected zero exit code {}",
        &output.status.code().unwrap()
    );
}

/// Asserts process has correct exit code and finished without an error
pub fn assert_process_exited_successfully(output: Output) {
    println!("Asserting process exited sucessfully...");

    assert!(
        output.status.success(),
        "non-zero exit code {}",
        &output.status.code().unwrap()
    );
}

pub fn assert_process_failed_and_contains_message(command: Command, expected_part: &str) {
    let output = process_utils::run_process_and_get_output(command);
    let actual = output.err_as_single_line();

    assert_eq!(
        actual.contains(&expected_part),
        true,
        "message : '{}' does not contain expected part '{}'",
        &actual,
        &expected_part
    );

    assert_process_failed(output);
}

pub fn assert_process_failed_and_contains_message_with_desc(
    command: Command,
    expected_part: &str,
    description: &str,
) {
    let output = process_utils::run_process_and_get_output(command);
    let actual = output.err_as_single_line();

    assert_eq!(
        actual.contains(&expected_part),
        true,
        "message : '{}' does not contain expected part '{}'. {}",
        &actual,
        &expected_part,
        &description
    );

    assert_process_failed(output);
}

pub fn assert_process_failed_and_matches_message(command: Command, expected_part: &str) {
    let output = process_utils::run_process_and_get_output(command);
    let actual = output.err_as_single_line();

    let re = Regex::new(expected_part).unwrap();

    assert_eq!(
        re.is_match(&actual),
        true,
        "message : '{}' does not match expected regex '{}'",
        &actual,
        &expected_part,
    );

    assert_process_failed(output);
}
