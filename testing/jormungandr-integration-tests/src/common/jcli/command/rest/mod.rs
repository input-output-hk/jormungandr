pub mod v0;

pub use v0::V0Command;

use assert_cmd::assert::OutputAssertExt;
use jortestkit::prelude::ProcessOutput;
use std::process::Command;

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
}
