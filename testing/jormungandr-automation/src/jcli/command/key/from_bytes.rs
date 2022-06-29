use std::{path::Path, process::Command};
pub struct KeyFromBytesCommand {
    command: Command,
}

impl KeyFromBytesCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn input<P: AsRef<Path>>(mut self, input: P) -> Self {
        self.command.arg(input.as_ref());
        self
    }

    pub fn key_type<S: Into<String>>(mut self, key_type: S) -> Self {
        self.command.arg("--type").arg(key_type.into());
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}
