use std::{path::Path, process::Command};

pub struct CommunicationKeyCommand {
    command: Command,
}

impl CommunicationKeyCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_public<P: AsRef<Path>, Q: AsRef<Path>>(
        mut self,
        private_key_file: P,
        output_file: Q,
    ) -> Self {
        self.command
            .arg("to-public")
            .arg("--input")
            .arg(private_key_file.as_ref())
            .arg(output_file.as_ref());
        self
    }

    pub fn generate(mut self) -> Self {
        self.command.arg("generate");
        self
    }

    pub fn build(self) -> Command {
        println!("{:?}", self.command);
        self.command
    }
}
