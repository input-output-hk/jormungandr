use crate::rest::{Error, RestArgs};
#[cfg(feature = "structopt")]
use structopt::StructOpt;

/// Shutdown node
#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub enum Shutdown {
    Post {
        #[cfg_attr(feature = "structopt", structopt(flatten))]
        args: RestArgs,
    },
}

impl Shutdown {
    pub fn exec(self) -> Result<(), Error> {
        let Shutdown::Post { args } = self;
        args.client()?.get(&["v0", "shutdown"]).execute()?;
        println!("Success");
        Ok(())
    }
}
