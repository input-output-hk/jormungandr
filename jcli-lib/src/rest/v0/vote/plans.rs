use crate::rest::{Error, RestArgs};
use crate::utils::OutputFormat;
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub enum Plans {
    /// Get active vote plans list
    Get {
        #[cfg_attr(feature = "structopt", structopt(flatten))]
        args: RestArgs,
        #[cfg_attr(feature = "structopt", structopt(flatten))]
        output_format: OutputFormat,
    },
}

impl Plans {
    pub fn exec(self) -> Result<(), Error> {
        let Plans::Get {
            args,
            output_format,
        } = self;
        let response = args
            .client()?
            .get(&["v0", "vote", "active", "plans"])
            .execute()?
            .json()?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
