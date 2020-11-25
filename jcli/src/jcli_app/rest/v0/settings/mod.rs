use crate::jcli_app::rest::{Error, RestArgs};
use crate::jcli_app::utils::OutputFormat;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Settings {
    /// Get node settings
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl Settings {
    pub fn exec(self) -> Result<(), Error> {
        let Settings::Get {
            args,
            output_format,
        } = self;
        let response =
            args.request_json_with_args(&["v0", "settings"], |client, url| client.get(url))?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
