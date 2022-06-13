mod from_bytes;
mod generate;
mod to_bytes;
mod to_public;

pub use from_bytes::KeyFromBytesCommand;
pub use generate::KeyGenerateCommand;
use std::process::Command;
pub use to_bytes::KeyToBytesCommand;
pub use to_public::KeyToPublicCommand;

pub struct KeyCommand {
    command: Command,
}

#[allow(clippy::wrong_self_convention)]
impl KeyCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn generate(mut self) -> KeyGenerateCommand {
        self.command.arg("generate");
        KeyGenerateCommand::new(self.command)
    }

    pub fn to_bytes(mut self) -> KeyToBytesCommand {
        self.command.arg("to-bytes");
        KeyToBytesCommand::new(self.command)
    }

    pub fn from_bytes(mut self) -> KeyFromBytesCommand {
        self.command.arg("from-bytes");
        KeyFromBytesCommand::new(self.command)
    }

    pub fn to_public(mut self) -> KeyToPublicCommand {
        self.command.arg("to-public");
        KeyToPublicCommand::new(self.command)
    }
}
