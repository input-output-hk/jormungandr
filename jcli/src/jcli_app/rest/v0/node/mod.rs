mod stats;

use self::stats::Stats;
use jcli_app::rest::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
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
