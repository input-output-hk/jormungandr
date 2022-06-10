mod account;
mod info;
mod single;

pub use account::AccountCommand;
pub use info::InfoCommand;
pub use single::SingleCommand;
use std::process::Command;
pub struct AddressCommand {
    command: Command,
}

impl AddressCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn account(mut self) -> AccountCommand {
        self.command.arg("account");
        AccountCommand::new(self.command)
    }

    pub fn info(mut self) -> InfoCommand {
        self.command.arg("info");
        InfoCommand::new(self.command)
    }

    pub fn single(mut self) -> SingleCommand {
        self.command.arg("single");
        SingleCommand::new(self.command)
    }
}
