use std::process::Command;

pub struct UtxOCommand {
    command: Command,
}

impl UtxOCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn get(mut self) -> Self {
        self.command.arg("get");
        self
    }

    pub fn host<S: Into<String>>(mut self, host: S) -> Self {
        self.command.arg("--host").arg(host.into());
        self
    }

    pub fn fragment_id<S: Into<String>>(mut self, fragment_id: S) -> Self {
        self.command.arg(fragment_id.into());
        self
    }

    pub fn output_index(mut self, output_index: u8) -> Self {
        self.command.arg(output_index.to_string());
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}
