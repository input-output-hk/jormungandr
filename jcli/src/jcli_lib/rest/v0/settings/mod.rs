use crate::jcli_lib::{
    rest::{Error, RestArgs},
    utils::OutputFormat,
};
use jormungandr_lib::interfaces::SettingsDto;
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
        let settings = request_settings(args)?;
        let formatted = output_format.format_json(serde_json::to_value(&settings)?)?;
        println!("{}", formatted);
        Ok(())
    }
}

pub fn request_settings(args: RestArgs) -> Result<SettingsDto, Error> {
    serde_json::from_str(&(args.client()?.get(&["v0", "settings"]).execute()?.text()?))
        .map_err(Error::SerdeError)
}
