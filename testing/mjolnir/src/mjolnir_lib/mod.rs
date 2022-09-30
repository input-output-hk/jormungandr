pub mod args;
pub mod bootstrap;
pub mod error;
pub mod explorer;
pub mod fragment;
pub mod generators;
pub mod rest;

pub use error::MjolnirError;
use jortestkit::{load::Monitor, prelude::ProgressBarMode};
use std::error::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Mjolnir {
    /// display full version details (software version, source version, targets and compiler used)
    #[structopt(long = "full-version")]
    full_version: bool,

    /// display the sources version, allowing to check the source's hash used to compile this executable.
    /// this option is useful for scripting retrieving the logs of the version of this application.
    #[structopt(long = "source-version")]
    source_version: bool,

    #[structopt(subcommand)]
    command: Option<MjolnirCommand>,
}

#[allow(clippy::large_enum_variant)]
/// Jormungandr Load CLI toolkit
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum MjolnirCommand {
    /// Passive Nodes bootstrap
    Passive(bootstrap::ClientLoadCommand),
    /// Explorer load
    Explorer(explorer::ExplorerLoadCommand),
    /// Fragment load
    Fragment(fragment::FragmentLoadCommand),
    /// Rest load
    Rest(rest::RestLoadCommand),
}

impl Mjolnir {
    pub fn exec(self) -> Result<(), Box<dyn Error>> {
        use std::io::Write as _;
        if self.full_version {
            Ok(writeln!(std::io::stdout(), "{}", env!("FULL_VERSION"))?)
        } else if self.source_version {
            Ok(writeln!(std::io::stdout(), "{}", env!("SOURCE_VERSION"))?)
        } else if let Some(cmd) = self.command {
            cmd.exec()
        } else {
            writeln!(std::io::stderr(), "No command, try `--help'")?;
            std::process::exit(1);
        }
    }
}

impl MjolnirCommand {
    pub fn exec(self) -> Result<(), Box<dyn Error>> {
        use self::MjolnirCommand::*;
        match self {
            Passive(bootstrap) => bootstrap.exec()?,
            Explorer(explorer) => explorer.exec()?,
            Fragment(fragment) => fragment.exec()?,
            Rest(rest) => rest.exec()?,
        };
        Ok(())
    }
}

pub fn build_monitor(progress_bar_mode: &ProgressBarMode) -> Monitor {
    match progress_bar_mode {
        ProgressBarMode::Monitor => Monitor::Progress(100),
        ProgressBarMode::Standard => Monitor::Standard(100),
        ProgressBarMode::None => Monitor::Disabled(10),
    }
}
