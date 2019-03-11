mod command_arguments;
pub mod init;
pub mod logging;
pub mod start;

pub use self::command_arguments::GenPrivKeyType;

use self::command_arguments::{Command as ArgCommand, CommandLine, GeneratePrivKeyArguments};
use std::fmt::{Display, Formatter, Result as FmtResult};

pub enum Command {
    Start(start::Settings),
    Init(init::Settings),
    GenerateKeys,
    GeneratePrivKey(GeneratePrivKeyArguments),
    GeneratePubKey,
}

#[derive(Debug)]
pub enum Error {
    Start(start::Error),
    Init(init::Error),
}

impl Command {
    pub fn load() -> Result<Self, Error> {
        let command_line = CommandLine::load();

        match command_line.command {
            ArgCommand::Init(ref args) => init::Settings::load(&command_line, args)
                .map(Command::Init)
                .map_err(Error::Init),
            ArgCommand::Start(ref args) => start::Settings::load(&command_line, args)
                .map(Command::Start)
                .map_err(Error::Start),
            ArgCommand::GenerateKeys => Ok(Command::GenerateKeys),
            ArgCommand::GeneratePrivKey(args) => Ok(Command::GeneratePrivKey(args)),
            ArgCommand::GeneratePubKey => Ok(Command::GeneratePubKey),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Error::Start(error) => write!(f, "{}", error),
            Error::Init(error) => write!(f, "{}", error),
        }
    }
}
