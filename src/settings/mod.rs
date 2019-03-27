mod command_arguments;
pub mod logging;
pub mod start;

pub use self::command_arguments::GenPrivKeyType;
pub use self::start::Error;

use self::command_arguments::{
    Command as ArgCommand, CommandLine, GeneratePrivKeyArguments, GeneratePubKeyArguments,
};

pub enum Command {
    Start(start::Settings),
    GeneratePrivKey(GeneratePrivKeyArguments),
    GeneratePubKey(GeneratePubKeyArguments),
}

impl Command {
    pub fn load() -> Result<Self, Error> {
        let command_line = CommandLine::load();

        match command_line.command {
            ArgCommand::Start(ref args) => {
                start::Settings::load(&command_line, args).map(Command::Start)
            }
            ArgCommand::GeneratePrivKey(args) => Ok(Command::GeneratePrivKey(args)),
            ArgCommand::GeneratePubKey(args) => Ok(Command::GeneratePubKey(args)),
        }
    }
}
