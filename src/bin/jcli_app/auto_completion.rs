use std::path::PathBuf;
use structopt::{clap::Shell, StructOpt};

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AutoCompletion {
    /// set the type shell for the auto completion output (bash, zsh...)
    shell: Shell,

    /// path to the directory to write the generated auto completion files
    output: PathBuf,
}

impl AutoCompletion {
    pub fn exec<S: StructOpt>(self) -> Result<(), Error> {
        validate_output(&self.output)?;
        S::clap().gen_completions("jcli", self.shell, self.output);
        Ok(())
    }
}

fn validate_output(output: &PathBuf) -> Result<(), Error> {
    if !output.exists() {
        return Err(Error::OutputNotExist);
    }
    if !output.is_dir() {
        return Err(Error::OutputNotDir);
    }
    Ok(())
}

custom_error! {pub Error
    OutputNotExist = "output directory does not exist",
    OutputNotDir = "output is not a directory",
}
