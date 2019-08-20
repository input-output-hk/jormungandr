use bawawa::{Command, Program};
use error_chain::ChainedError as _;

const JORMUNGANDR_PROGRAM_NAME: &str = "jormungandr";
const JCLI_PROGRAM_NAME: &str = "jcli";

lazy_static! {
    /// this is a bit of a hack to try to get the binary directory
    /// of the executable we are testing.
    ///
    /// it should resolve to the `/target/{profile}/`
    static ref BIN_DIRECTORY: std::path::PathBuf = {
        let mut output_directory = std::env::current_exe().unwrap();

        output_directory.pop();
        output_directory.pop();
        output_directory
    };
}

lazy_static! {
    /// the bawawa `Command` for `jormungandr`.
    pub static ref JORMUNGANDR: Command = { prepare_command(JORMUNGANDR_PROGRAM_NAME) };
    /// the bawawa `Command` for `jcli`.
    pub static ref JCLI: Command = { prepare_command(JCLI_PROGRAM_NAME) };
}

/// internal function to prepare a bawawa `Command` for `jormungandr` and `jcli`
///
/// if the program could not be found in the $PATH or the current path then this
/// function will print the error reported by `bawawa` and then will `panic!` so
/// the tests are not executed.
fn prepare_command(program_name: &str) -> Command {
    let mut exe = BIN_DIRECTORY.clone();
    exe.push(program_name);
    match Program::new(exe.display().to_string()) {
        Ok(program) => Command::new(program),
        Err(error) => {
            eprintln!("{}", error.display_chain().to_string());
            panic!(
                "the program {} is necessary for the execution of the tests but could not be found in  {}",
                program_name,
                BIN_DIRECTORY.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bawawa::{Process, Result, StandardOutput};
    use tokio::prelude::*;

    /// this tests `jormungandr` version is consistent with the version
    /// we know of.
    ///
    /// This is mainly testing we are testing the appropriate version
    /// of jormungandr. However this test is not *very* strong.
    ///
    #[test]
    fn check_jormungandr_version() -> Result<()> {
        let mut cmd = JORMUNGANDR.clone();
        cmd.arguments(&["--version"]);

        let mut captured_version = Process::spawn(cmd)?
            .capture_stdout(tokio::codec::LinesCodec::new())
            .wait();

        assert_eq!(
            captured_version.next().unwrap()?,
            format!("{} {}", JORMUNGANDR_PROGRAM_NAME, env!("CARGO_PKG_VERSION"))
        );

        Ok(())
    }

    /// this tests `jcli` version is consistent with the version
    /// we know of.
    ///
    /// This is mainly testing we are testing the appropriate version
    /// of jcli. However this test is not *very* strong.
    ///
    #[test]
    fn check_jcli_version() -> Result<()> {
        let mut cmd = JCLI.clone();
        cmd.arguments(&["--version"]);

        let mut captured_version = Process::spawn(cmd)?
            .capture_stdout(tokio::codec::LinesCodec::new())
            .wait();

        assert_eq!(
            captured_version.next().unwrap()?,
            format!("{} {}", JCLI_PROGRAM_NAME, env!("CARGO_PKG_VERSION"))
        );

        Ok(())
    }
}
