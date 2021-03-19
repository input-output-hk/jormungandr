mod voters;

use structopt::StructOpt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("error while writing to csv")]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    Other(#[from] crate::jcli_app::block::Error),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Rewards {
    /// Calculate rewards for voters base on their stake
    Voters(voters::VotersRewards),
}

impl Rewards {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Rewards::Voters(cmd) => cmd.exec(),
        }
    }
}
