use crate::rest::{Error, RestArgs};
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub enum Diagnostic {
    /// Get system diagnostic information
    Get {
        #[cfg_attr(feature = "structopt", structopt(flatten))]
        args: RestArgs,
    },
}

impl Diagnostic {
    pub fn exec(self) -> Result<(), Error> {
        let args = match self {
            Diagnostic::Get { args } => args,
        };
        let response = args
            .client()?
            .get(&["v0", "diagnostic"])
            .execute()?
            .text()?;
        println!("{}", response);
        Ok(())
    }
}
