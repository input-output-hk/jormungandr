mod active;
mod committees;
mod plans;

use self::active::Active;
use crate::rest::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "active", rename_all = "kebab-case")]
pub enum Vote {
    /// Active vote related operations
    Active(Active),
}

impl Vote {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Vote::Active(active) => active.exec(),
        }
    }
}
