use std::process::Command;

mod address;
mod certificate;
mod genesis;
mod key;
pub mod rest;
mod transaction;
pub mod votes;

pub use address::AddressCommand;
pub use certificate::CertificateCommand;
pub use genesis::GenesisCommand;
pub use key::KeyCommand;
pub use rest::RestCommand;
pub use transaction::TransactionCommand;
pub use votes::VotesCommand;

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

    pub fn rest(mut self) -> RestCommand {
        self.command.arg("rest");
        RestCommand::new(self.command)
    }

    pub fn transaction(mut self) -> TransactionCommand {
        self.command.arg("transaction");
        TransactionCommand::new(self.command)
    }

    pub fn certificate(mut self) -> CertificateCommand {
        self.command.arg("certificate");
        CertificateCommand::new(self.command)
    }

    pub fn votes(mut self) -> VotesCommand {
        self.command.arg("votes");
        VotesCommand::new(self.command)
    }
}
