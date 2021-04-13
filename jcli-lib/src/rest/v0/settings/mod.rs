use crate::rest::{Error, RestArgs};
use crate::utils::OutputFormat;
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
        let response = args.client()?.get(&["v0", "settings"]).execute()?.json()?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
