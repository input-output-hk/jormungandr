use crate::jcli_app::rest::{config::RestArgs, Error};
use crate::jcli_app::utils::OutputFormat;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Plans {
    /// Get active vote plans list
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        #[structopt(flatten)]
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
            .request_json_with_args(&["v0", "vote", "active", "plans"], |client, url| {
                client.get(url)
            })?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
