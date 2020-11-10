use std::process::Command;

mod communication_key;
mod member_key;

pub use communication_key::CommunicationKeyCommand;
pub use member_key::MemberKeyCommand;

pub struct CommitteeCommand {
    command: Command,
}

impl CommitteeCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn member_key(mut self) -> MemberKeyCommand {
        self.command.arg("member-key");
        MemberKeyCommand::new(self.command)
    }

    pub fn communication_key(mut self) -> CommunicationKeyCommand {
        self.command.arg("communication-key");
        CommunicationKeyCommand::new(self.command)
    }
}
