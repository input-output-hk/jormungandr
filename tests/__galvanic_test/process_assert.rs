use std::process::Child;
use std::process::Command;

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
pub fn assert_process_exited_successfully(mut command: Command, description: &str) {
    let mut process =  command
            .spawn()
            .expect(&format!("failed to execute {} command",&description));

    let exit_code = process
            .wait()
            .expect(&format!("failed to wait for {} to finish",&description));

    assert!(exit_code.success(),"non-zero exit code {} of command {}",
            &exit_code.code().unwrap(),
            &description);
}