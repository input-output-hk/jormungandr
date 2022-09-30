use crate::jcli_lib::{
    rest::{Error, RestArgs},
    utils::OutputFormat,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Stake {
    /// Get stake distribution
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        #[structopt(flatten)]
        output_format: OutputFormat,
        /// Epoch to get the stake distribution from
        epoch: Option<u32>,
    },
}

impl Stake {
    pub fn exec(self) -> Result<(), Error> {
        let Stake::Get {
            args,
            output_format,
            epoch,
        } = self;
        let epoch = epoch.map(|epoch| epoch.to_string());
        let mut url = vec!["v0", "stake"];
        if let Some(epoch) = &epoch {
            url.push(epoch);
        }
        let response = args.client()?.get(&url).execute()?.json()?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
