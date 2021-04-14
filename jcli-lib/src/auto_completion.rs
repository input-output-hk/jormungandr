//! Generated auto-completions for supported shells supported by `structopt` via `clap`.
use std::path::{Path, PathBuf};
#[cfg(feature = "structopt")]
use structopt::{clap::Shell, StructOpt};
use thiserror::Error;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub struct AutoCompletion {
    #[cfg(feature = "structopt")]
    /// set the type shell for the auto completion output (bash, zsh...)
    shell: Shell,

    /// path to the directory to write the generated auto completion files
    output: PathBuf,
}

#[cfg(feature = "structopt")]
impl AutoCompletion {
    pub fn exec<S: StructOpt>(self) -> Result<(), Error> {
        validate_output(&self.output)?;
        S::clap().gen_completions("jcli", self.shell, self.output);
        Ok(())
    }
}

fn validate_output(output: &Path) -> Result<(), Error> {
    if !output.exists() {
        return Err(Error::OutputNotExist);
    }
    if !output.is_dir() {
        return Err(Error::OutputNotDir);
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("output directory does not exist")]
    OutputNotExist,
    #[error("output is not a directory")]
    OutputNotDir,
}
