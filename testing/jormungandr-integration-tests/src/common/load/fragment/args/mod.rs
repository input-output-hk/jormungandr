use crate::common::load::ClientLoadError;
use structopt::StructOpt;
use thiserror::Error;

mod batch;
mod standard;

#[derive(StructOpt, Debug)]
pub enum FragmentLoadCommand {
    /// Prints nodes related data, like stats,fragments etc.
    Batch(batch::Batch),
    /// Spawn leader or passive node (also legacy)
    Standard(standard::Standard),
}

#[derive(Error, Debug)]
pub enum FragmentLoadCommandError {
    #[error("No scenario defined for run. Available: [duration,iteration]")]
    NoScenarioDefined,
    #[error("Client Error")]
    ClientError(#[from] ClientLoadError),
}

impl FragmentLoadCommand {
    pub fn exec(&self) -> Result<(), FragmentLoadCommandError> {
        match self {
            FragmentLoadCommand::Batch(batch) => batch.exec(),
            FragmentLoadCommand::Standard(standard) => standard.exec(),
        }
    }
}
