use std::process::Command;

pub struct NodeCommand {
    command: Command,
}

impl NodeCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn stats<S: Into<String>>(mut self, host: S) -> Self {
        self.command
            .arg("stats")
            .arg("get")
            .arg("-h")
            .arg(host.into());
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}
