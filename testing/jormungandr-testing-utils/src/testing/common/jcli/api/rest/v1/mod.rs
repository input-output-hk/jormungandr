mod vote;

use vote::Vote;

use crate::testing::common::jcli::command::rest::V1Command;

pub struct RestV1 {
    v1_command: V1Command,
}

impl RestV1 {
    pub fn new(v1_command: V1Command) -> Self {
        Self { v1_command }
    }

    pub fn vote(self) -> Vote {
        Vote::new(self.v1_command.vote())
    }
}
