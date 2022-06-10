mod vote;

use std::process::Command;
pub use vote::VoteCommand;

pub struct V1Command {
    command: Command,
}

impl V1Command {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn vote(mut self) -> VoteCommand {
        self.command.arg("vote");
        VoteCommand::new(self.command)
    }

    pub fn build(self) -> Command {
        println!("{:?}", self.command);
        self.command
    }
}
