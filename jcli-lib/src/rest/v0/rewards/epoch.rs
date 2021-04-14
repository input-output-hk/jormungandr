use crate::rest::{Error, RestArgs};
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub enum Epoch {
    /// Get rewards for epoch
    Get {
        #[cfg_attr(feature = "structopt", structopt(flatten))]
        args: RestArgs,
        /// Epoch number
        epoch: u32,
    },
}

impl Epoch {
    pub fn exec(self) -> Result<(), Error> {
        let Epoch::Get { args, epoch } = self;
        let response = args
            .client()?
            .get(&["v0", "rewards", "epoch", &epoch.to_string()])
            .execute()?
            .text()?;
        println!("{}", response);
        Ok(())
    }
}
