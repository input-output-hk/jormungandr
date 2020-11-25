use crate::jcli_app::rest::{config::RestArgs, Error};
use crate::jcli_app::utils::OutputFormat;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum StakePools {
    /// Get stake pool IDs
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl StakePools {
    pub fn exec(self) -> Result<(), Error> {
        let StakePools::Get {
            args,
            output_format,
        } = self;
        let response =
            args.request_json_with_args(&["v0", "stake_pools"], |client, url| client.get(url))?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
