use std::process::Command;
pub struct KeyGenerateCommand {
    command: Command,
}

impl KeyGenerateCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn key_type<S: Into<String>>(mut self, key_type: S) -> Self {
        self.command.arg("--type").arg(key_type.into());
        self
    }

    pub fn seed<S: Into<String>>(mut self, seed: S) -> Self {
        self.command.arg("--seed").arg(seed.into());
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}
