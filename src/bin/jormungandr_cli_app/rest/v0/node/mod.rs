mod stats;

use self::stats::Stats;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Node {
    /// Node information
    Stats(Stats),
}

impl Node {
    pub fn exec(self) {
        match self {
            Node::Stats(stats) => stats.exec(),
        }
    }
}
