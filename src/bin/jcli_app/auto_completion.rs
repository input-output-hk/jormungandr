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
    pub fn exec<S: StructOpt>(self) {
        S::clap().gen_completions("jcli", self.shell, self.output)
    }
}
