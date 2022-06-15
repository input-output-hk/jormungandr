use crate::jcli_lib::{
    rest::{Error, RestArgs},
    utils::OutputFormat,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Committees {
    /// Get committee members list
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl Committees {
    pub fn exec(self) -> Result<(), Error> {
        let Committees::Get {
            args,
            output_format,
        } = self;
        let response = args
            .client()?
            .get(&["v0", "vote", "active", "committees"])
            .execute()?
            .json()?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
