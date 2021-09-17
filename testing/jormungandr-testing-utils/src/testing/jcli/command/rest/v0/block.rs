use std::process::Command;

pub struct BlockCommand {
    command: Command,
}

impl BlockCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn get<P: Into<String>, S: Into<String>>(mut self, block_id: P, host: S) -> Self {
        self.command
            .arg(block_id.into())
            .arg("get")
            .arg("-h")
            .arg(host.into());
        self
    }
    pub fn next<P: Into<String>, S: Into<String>>(
        mut self,
        block_id: P,
        limit: u32,
        host: S,
    ) -> Self {
        self.command
            .arg(block_id.into())
            .arg("next-id")
            .arg("get")
            .arg("--count")
            .arg(limit.to_string())
            .arg("-h")
            .arg(host.into());
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}
