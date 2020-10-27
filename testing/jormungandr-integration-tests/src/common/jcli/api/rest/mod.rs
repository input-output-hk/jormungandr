mod v0;

use v0::RestV0;

use crate::common::jcli::command::RestCommand;

pub struct Rest {
    rest_command: RestCommand,
}

impl Rest {
    pub fn new(rest_command: RestCommand) -> Self {
        Self { rest_command }
    }

    pub fn v0(self) -> RestV0 {
        RestV0::new(self.rest_command.v0())
    }
}
