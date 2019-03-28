mod command_arguments;
pub mod logging;
pub mod start;

pub use self::start::Error;

use self::command_arguments::{Command as ArgCommand, CommandLine};

pub enum Command {
    Start(start::Settings),
}

impl Command {
    pub fn load() -> Result<Self, Error> {
        let command_line = CommandLine::load();

        match command_line.command {
            ArgCommand::Start(ref args) => {
                start::Settings::load(&command_line, args).map(Command::Start)
            }
        }
    }
}
