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

        attempts += 1;
        thread::sleep(sleep_between_attempt_duration);
    }
}

pub fn sleep(seconds: u64) {
    let duration = time::Duration::from_secs(seconds);
    thread::sleep(duration);
}
