use jormungandr_automation::jormungandr::{explorer::Explorer, ExplorerError};
use std::{env, error::Error as _};
use structopt::StructOpt;

fn main() {
    Command::from_args().exec().unwrap_or_else(report_error)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Explorer(#[from] ExplorerError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn report_error(error: Error) {
    eprintln!("{}", error);
    let mut source = error.source();
    while let Some(sub_error) = source {
        eprintln!("  |-> {}", sub_error);
        source = sub_error.source();
    }
    std::process::exit(1)
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Command {
    /// display full version details (software version, source version, targets and compiler used)
    #[structopt(long = "full-version")]
    full_version: bool,

    /// display the sources version, allowing to check the source's hash used to compile this executable.
    /// this option is useful for scripting retrieving the logs of the version of this application.
    #[structopt(long = "source-version")]
    source_version: bool,

    /// explorer address
    #[structopt(long = "address")]
    address: String,

    #[structopt(subcommand)]
    command: Option<ExplorerClientCommand>,
}

#[allow(clippy::large_enum_variant)]
/// Explorer Client CLI toolkit
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum ExplorerClientCommand {
    /// Get last block
    LastBlock,
}

impl Command {
    pub fn exec(self) -> Result<(), Error> {
        use std::io::Write as _;
        if self.full_version {
            Ok(writeln!(std::io::stdout(), "{}", env!("FULL_VERSION"))?)
        } else if self.source_version {
            Ok(writeln!(std::io::stdout(), "{}", env!("SOURCE_VERSION"))?)
        } else if let Some(cmd) = self.command {
            cmd.exec(self.address).map_err(Into::into)
        } else {
            writeln!(std::io::stderr(), "No command, try `--help'")?;
            std::process::exit(1);
        }
    }
}

impl ExplorerClientCommand {
    pub fn exec(self, address: String) -> Result<(), ExplorerError> {
        let mut explorer = Explorer::new(address);
        explorer.disable_logs();
        match self {
            Self::LastBlock => println!("{:#?}", explorer.last_block()?),
        };
        Ok(())
    }
}
