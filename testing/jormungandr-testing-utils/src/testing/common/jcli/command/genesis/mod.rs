mod decode;
mod encode;
mod hash;

pub use decode::GenesisDecodeCommand;
pub use encode::GenesisEncodeCommand;
pub use hash::GenesisHashCommand;
use std::process::Command;
pub struct GenesisCommand {
    command: Command,
}

impl GenesisCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn encode(mut self) -> GenesisEncodeCommand {
        self.command.arg("encode");
        GenesisEncodeCommand::new(self.command)
    }

    pub fn decode(mut self) -> GenesisDecodeCommand {
        self.command.arg("decode");
        GenesisDecodeCommand::new(self.command)
    }

    pub fn hash(mut self) -> GenesisHashCommand {
        self.command.arg("hash");
        GenesisHashCommand::new(self.command)
    }

    pub fn init(mut self) -> Command {
        self.command.arg("init");
        self.command
    }
}
