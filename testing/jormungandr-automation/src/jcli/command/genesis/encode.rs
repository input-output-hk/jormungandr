use std::{path::Path, process::Command};
pub struct GenesisEncodeCommand {
    command: Command,
}

impl GenesisEncodeCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn input<P: AsRef<Path>>(mut self, input: P) -> Self {
        self.command.arg("--input").arg(input.as_ref());
        self
    }

    pub fn output<P: AsRef<Path>>(mut self, output: P) -> Self {
        self.command.arg("--output").arg(output.as_ref());
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}
