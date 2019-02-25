mod command_arguments;
pub mod init;
pub mod logging;
pub mod start;

pub enum Command {
    Start(start::Settings),
    Init(init::Settings),
    GenerateKeys,
}

#[derive(Debug)]
pub enum Error {
    Start(start::Error),
    Init(init::Error),
    GenerateKeys,
}

impl Command {
    pub fn load() -> Result<Self, Error> {
        let command_line = command_arguments::CommandLine::load();

        match command_line.command {
            command_arguments::Command::Init(ref options) => {
                init::Settings::load(&command_line, options)
                    .map(Command::Init)
                    .map_err(Error::Init)
            }
            command_arguments::Command::Start(ref options) => {
                start::Settings::load(&command_line, options)
                    .map(Command::Start)
                    .map_err(Error::Start)
            }
            command_arguments::Command::GenerateKeys => Ok(Command::GenerateKeys),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Start(error) => std::fmt::Display::fmt(error, f),
            Error::Init(error) => std::fmt::Display::fmt(error, f),
            Error::GenerateKeys => write!(f, "error in the generate-keys command"),
        }
    }
}
