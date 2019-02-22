mod command_arguments;
pub mod logging;
pub mod start;

use crate::blockcfg::genesis_data::*;

pub enum Command {
    Start(start::Settings),
}

#[derive(Debug)]
pub enum Error {
    Start(start::Error),
}

impl Command {
    pub fn load() -> Result<Self, Error> {
        let command_line = command_arguments::CommandLine::load();

        match command_line.command {
            command_arguments::Command::Init(_) => unimplemented!(),
            command_arguments::Command::Start(ref options) => {
                start::Settings::load(&command_line, options)
                    .map(Command::Start)
                    .map_err(Error::Start)
            }
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Start(error) => std::fmt::Display::fmt(error, f),
        }
    }
}
