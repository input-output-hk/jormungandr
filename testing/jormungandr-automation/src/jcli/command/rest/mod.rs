pub mod v0;
pub mod v1;

use std::process::Command;
pub use v0::V0Command;
pub use v1::V1Command;

pub struct RestCommand {
    command: Command,
}

impl RestCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn v0(mut self) -> V0Command {
        self.command.arg("v0");
        V0Command::new(self.command)
    }

    pub fn v1(mut self) -> V1Command {
        self.command.arg("v1");
        V1Command::new(self.command)
    }
}
