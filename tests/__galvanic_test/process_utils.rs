use std::process::{Child, Command};
use std::{thread, time};

/// Run command for n times with m second interval.
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
}

/// Struct ensures child process is killed if leaves given scope
///
pub struct ProcessKillGuard {
    child: Child,
}

impl ProcessKillGuard {
    pub fn new(child: Child) -> ProcessKillGuard {
        ProcessKillGuard { child }
    }
}

impl Drop for ProcessKillGuard {
    fn drop(&mut self) {
        match self.child.kill() {
            Err(e) => println!("Could not kill child process: {}", e),
            Ok(_) => println!("Successfully killed child process"),
        }
    }
}
