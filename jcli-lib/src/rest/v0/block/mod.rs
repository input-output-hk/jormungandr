use crate::rest::Error;
#[cfg(feature = "structopt")]
use structopt::StructOpt;

mod next_id;
mod subcommand;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub struct Block {
    /// ID of the block
    block_id: String,

    #[cfg_attr(feature = "structopt", structopt(subcommand))]
    subcommand: subcommand::Subcommand,
}

impl Block {
    pub fn exec(self) -> Result<(), Error> {
        self.subcommand.exec(self.block_id)
    }
}
