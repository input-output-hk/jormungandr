use crate::rest::{Error, RestArgs};
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub enum Tip {
    /// Get tip ID
    Get {
        #[cfg_attr(feature = "structopt", structopt(flatten))]
        args: RestArgs,
    },
}

impl Tip {
    pub fn exec(self) -> Result<(), Error> {
        let args = match self {
            Tip::Get { args } => args,
        };
        let response = args.client()?.get(&["v0", "tip"]).execute()?.text()?;
        println!("{}", response);
        Ok(())
    }
}
