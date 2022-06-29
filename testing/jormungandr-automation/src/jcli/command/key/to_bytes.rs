use std::{path::Path, process::Command};
pub struct KeyToBytesCommand {
    command: Command,
}

impl KeyToBytesCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn input<P: AsRef<Path>>(mut self, input: P) -> Self {
        self.command.arg(input.as_ref());
        self
    }

    pub fn output<P: AsRef<Path>>(mut self, output: P) -> Self {
        self.command.arg(output.as_ref());
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}
