mod api;
mod command;

use command::JCliCommand;

use crate::common::configuration;
use api::{JCliGenesis, JCliKey};
use std::{path::PathBuf, process::Command};

#[derive(Clone, Debug)]
pub struct JCli {
    exe: PathBuf,
}

impl Default for JCli {
    fn default() -> Self {
        Self::new(configuration::get_jcli_app())
    }
}

impl JCli {
    pub fn new(exe: PathBuf) -> Self {
        Self { exe }
    }

    pub fn genesis(&self) -> JCliGenesis {
        let command = Command::new(self.exe.clone());
        let jcli_command = JCliCommand::new(command);
        JCliGenesis::new(jcli_command.genesis())
    }

    pub fn key(&self) -> JCliKey {
        let command = Command::new(self.exe.clone());
        let jcli_command = JCliCommand::new(command);
        JCliKey::new(jcli_command.key())
    }
}
