use std::process::Command;
pub struct EncryptingVoteKeyCommand {
    command: Command,
}

impl EncryptingVoteKeyCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn generate(mut self) -> Self {
        self.command.arg("generate");
        self
    }

    pub fn build(self) -> Command {
        println!("{:?}", self.command);
        self.command
    }
}
