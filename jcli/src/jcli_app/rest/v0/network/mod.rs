mod stats;

use self::stats::Stats;
use crate::jcli_app::rest::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Network {
    /// Network information
    Stats(Stats),
}

impl Network {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Network::Stats(stats) => stats.exec(),
        }
    }
}
