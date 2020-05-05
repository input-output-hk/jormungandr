use bawawa::{Command, Program};
use error_chain::ChainedError as _;
use std::path::PathBuf;

/// internal function to prepare a bawawa `Command` for `jormungandr` and `jcli`
///
/// if the program could not be found in the $PATH or the current path then this
/// function will print the error reported by `bawawa` and then will `panic!` so
/// the tests are not executed.
pub fn prepare_command(exe: PathBuf) -> Command {
    let cmd = match Program::new(exe.display().to_string()) {
        Ok(program) => Command::new(program),
        Err(error) => {
            eprintln!("{}", error.display_chain().to_string());
            panic!(
                "the program {} is necessary for the execution of the tests but could not be found",
                exe.display(),
            );
        }
    };

    check_command_version(cmd.clone());

    cmd
}

fn check_command_version(mut cmd: Command) {
    use bawawa::Process;
    use tokio::prelude::*;

    cmd.arguments(&["--version"]);

    let exit_status = Process::spawn(cmd.clone()).unwrap().wait().unwrap();

    assert!(
        exit_status.success(),
        "cannot execute the command successfully '{}'",
        cmd
    );
}
