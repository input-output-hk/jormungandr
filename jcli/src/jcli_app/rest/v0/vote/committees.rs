use crate::jcli_app::rest::{config::RestArgs, Error};
use crate::jcli_app::utils::OutputFormat;
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
            .request_json_with_args(&["v0", "vote", "active", "committees"], |client, url| {
                client.get(url)
            })?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
