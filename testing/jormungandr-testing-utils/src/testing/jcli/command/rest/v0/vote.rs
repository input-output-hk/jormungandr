use std::process::Command;

pub struct VoteCommand {
    command: Command,
}

impl VoteCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn active_committees<S: Into<String>>(mut self, host: S) -> Self {
        self.command
            .arg("active")
            .arg("committees")
            .arg("get")
            .arg("--host")
            .arg(host.into());
        self
    }

    pub fn active_vote_plans<S: Into<String>>(mut self, host: S) -> Self {
        self.command
            .arg("active")
            .arg("plans")
            .arg("get")
            .arg("--host")
            .arg(host.into());
        self
    }

    pub fn build(self) -> Command {
        println!("{:?}", self.command);
        self.command
    }
}
