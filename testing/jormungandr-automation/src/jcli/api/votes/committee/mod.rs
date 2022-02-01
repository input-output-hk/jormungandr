use crate::jcli::command::votes::CommitteeCommand;

mod communication_key;
mod member_key;

pub use communication_key::CommunicationKey;
pub use member_key::MemberKey;

pub struct Committee {
    committee_command: CommitteeCommand,
}

impl Committee {
    pub fn new(committee_command: CommitteeCommand) -> Self {
        Self { committee_command }
    }

    pub fn member_key(self) -> MemberKey {
        MemberKey::new(self.committee_command.member_key())
    }

    pub fn communication_key(self) -> CommunicationKey {
        CommunicationKey::new(self.committee_command.communication_key())
    }
}
