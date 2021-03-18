mod voters;

use jormungandr_lib::interfaces::Block0ConfigurationError;
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

type Error = crate::jcli_app::block::Error;

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
