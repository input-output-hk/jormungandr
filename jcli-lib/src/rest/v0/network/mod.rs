mod stats;

use self::stats::Stats;
use crate::rest::Error;
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
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
