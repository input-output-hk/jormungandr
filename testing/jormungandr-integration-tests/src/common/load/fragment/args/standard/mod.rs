mod all;
mod tx_only;

use super::FragmentLoadCommandError;
pub use all::AllFragments;
use structopt::StructOpt;
pub use tx_only::TxOnly;
#[derive(StructOpt, Debug)]
pub enum Standard {
    /// Put load on .
    TxOnly(tx_only::TxOnly),
    ///
    All(all::AllFragments),
}

impl Standard {
    pub fn exec(&self) -> Result<(), FragmentLoadCommandError> {
        match self {
            Standard::TxOnly(tx_only_command) => tx_only_command.exec(),
            Standard::All(all_command) => all_command.exec(),
        }
    }
}
