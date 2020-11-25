use crate::jcli_app::rest::{Error, RestArgs};
use crate::jcli_app::utils::OutputFormat;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Stats {
    /// Get network information
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
            .request_with_args(&["v0", "network", "stats"], |client, url| client.get(url))?
            .json()?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
