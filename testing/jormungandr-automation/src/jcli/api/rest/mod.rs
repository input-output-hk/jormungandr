mod v0;
mod v1;

use crate::jcli::command::RestCommand;
use v0::RestV0;
use v1::RestV1;

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

    pub fn v1(self) -> RestV1 {
        RestV1::new(self.rest_command.v1())
    }
}
