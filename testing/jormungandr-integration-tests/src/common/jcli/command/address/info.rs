use std::process::Command;
pub struct InfoCommand {
    command: Command,
}

impl InfoCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn address<S: Into<String>>(mut self, address: S) -> Self {
        self.command.arg(address.into());
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}
