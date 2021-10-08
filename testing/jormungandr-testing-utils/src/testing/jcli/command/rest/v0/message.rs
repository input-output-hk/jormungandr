use std::{path::Path, process::Command};

pub struct MessageCommand {
    command: Command,
}

impl MessageCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn post<P: AsRef<Path>, S: Into<String>>(mut self, transaction_file: P, host: S) -> Self {
        self.command
            .arg("post")
            .arg("--file")
            .arg(transaction_file.as_ref())
            .arg("--host")
            .arg(host.into());
        self
    }

    pub fn logs<S: Into<String>>(mut self, host: S) -> Self {
        self.command.arg("logs").arg("--host").arg(host.into());
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}
