use crate::jcli_app::rest::Error;
use structopt::StructOpt;

mod next_id;
mod subcommand;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Block {
    /// ID of the block
    block_id: String,

    #[structopt(subcommand)]
    subcommand: subcommand::Subcommand,
}

impl Block {
    pub fn exec(self) -> Result<(), Error> {
        self.subcommand.exec(self.block_id)
    }
}
