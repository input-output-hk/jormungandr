mod address;
mod auto_completion;
mod block;
mod certificate;
mod debug;
mod key;
mod rest;
mod rewards;
mod transaction;
mod vote;

pub mod utils;

use std::error::Error;
use structopt::StructOpt;

/// Jormungandr CLI toolkit
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct JCli {
    /// display full version details (software version, source version, targets and compiler used)
    #[structopt(long = "full-version")]
    full_version: bool,

    /// display the sources version, allowing to check the source's hash used to compile this executable.
    /// this option is useful for scripting retrieving the logs of the version of this application.
    #[structopt(long = "source-version")]
    source_version: bool,

    #[structopt(subcommand)]
    command: Option<JCliCommand>,
}

#[allow(clippy::large_enum_variant)]
/// Jormungandr CLI toolkit
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum JCliCommand {
    /// Key Generation
    Key(key::Key),
    /// Address tooling and helper
    Address(address::Address),
    /// Block tooling and helper
    Genesis(block::Genesis),
    /// Send request to node REST API
    Rest(rest::Rest),
    /// Build and view offline transaction
    Transaction(transaction::Transaction),
    /// Debug tools for developers
    Debug(debug::Debug),
    /// Certificate generation tool
    Certificate(certificate::Certificate),
    /// Auto completion
    AutoCompletion(auto_completion::AutoCompletion),
    /// Utilities that perform specialized tasks
    Utils(utils::Utils),
    /// Vote related operations
    Votes(vote::Vote),
    /// Rewards related operations
    Rewards(rewards::Rewards),
}

impl JCli {
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

impl JCliCommand {
    pub fn exec(self) -> Result<(), Box<dyn Error>> {
        use self::JCliCommand::*;
        match self {
            Key(key) => key.exec()?,
            Address(address) => address.exec()?,
            Genesis(genesis) => genesis.exec()?,
            Rest(rest) => rest.exec()?,
            Transaction(transaction) => transaction.exec()?,
            Debug(debug) => debug.exec()?,
            Certificate(certificate) => certificate.exec()?,
            AutoCompletion(auto_completion) => auto_completion.exec::<Self>()?,
            Utils(utils) => utils.exec()?,
            Votes(vote) => vote.exec()?,
            Rewards(rewards) => rewards.exec()?,
        };
        Ok(())
    }
}
