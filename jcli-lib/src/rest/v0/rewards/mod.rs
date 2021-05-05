mod epoch;
mod history;

use self::epoch::Epoch;
use self::history::History;

use crate::rest::Error;
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(name = "rewards", rename_all = "kebab-case")
)]
pub enum Rewards {
    /// Rewards distribution history one or more epochs starting from the last one
    History(History),
    /// Rewards distribution for a specific epoch
    Epoch(Epoch),
}

impl Rewards {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Rewards::History(history) => history.exec(),
            Rewards::Epoch(epoch) => epoch.exec(),
        }
    }
}
