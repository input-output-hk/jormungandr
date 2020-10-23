use std::process::Command;

mod address;
mod genesis;
mod key;

pub use address::AddressCommand;
pub use genesis::GenesisCommand;
pub use key::KeyCommand;

pub struct JCliCommand {
    command: Command,
}

impl JCliCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn genesis(mut self) -> GenesisCommand {
        self.command.arg("genesis");
        GenesisCommand::new(self.command)
    }

    pub fn key(mut self) -> KeyCommand {
        self.command.arg("key");
        KeyCommand::new(self.command)
    }

    pub fn address(mut self) -> AddressCommand {
        self.command.arg("address");
        AddressCommand::new(self.command)
    }
}
