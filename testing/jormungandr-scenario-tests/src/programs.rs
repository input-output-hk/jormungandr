use std::path::{Path, PathBuf};
use std::process::Command;

/// internal function to prepare an executable path name for `jormungandr` and `jcli`
///
/// if the program could not be found in the $PATH or the current path then this
/// function will panic so the tests are not executed.
pub fn prepare_command(exe: impl Into<PathBuf>) -> PathBuf {
    let exe = exe.into();
    check_command_version(&exe);
    exe
}

fn check_command_version(exe: &Path) {
    let mut cmd = Command::new(exe);
    cmd.arg("--version");

    let exit_status = cmd.spawn().unwrap().wait().unwrap();

    assert!(
        exit_status.success(),
        "cannot execute the command successfully: {:?}",
        cmd
    );
}
