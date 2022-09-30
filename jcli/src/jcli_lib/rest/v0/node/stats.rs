use crate::jcli_lib::{
    rest::{Error, RestArgs},
    utils::OutputFormat,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Stats {
    /// Get node information
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl Stats {
    pub fn exec(self) -> Result<(), Error> {
        let Stats::Get {
            args,
            output_format,
        } = self;
        let response = args
            .client()?
            .get(&["v0", "node", "stats"])
            .execute()?
            .json()?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
