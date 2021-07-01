use std::process::Command;
pub struct ElectionPublicKeyCommand {
    command: Command,
}

impl ElectionPublicKeyCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn generate(mut self) -> Self {
        self.command.arg("generate");
        self
    }

    pub fn build(self) -> Command {
        println!("{:?}", self.command);
        self.command
    }
}
