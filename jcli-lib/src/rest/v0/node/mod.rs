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
pub enum Node {
    /// Node information
    Stats(Stats),
}

impl Node {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Node::Stats(stats) => stats.exec(),
        }
    }
}
