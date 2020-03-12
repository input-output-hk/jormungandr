use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, OutputFormat, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Stake {
    /// Get stake distribution
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        #[structopt(flatten)]
        output_format: OutputFormat,
        /// Epoch to get the stake distribution from
        epoch: Option<u32>,
    },
}

impl Stake {
    pub fn exec(self) -> Result<(), Error> {
        let Stake::Get {
            addr,
            debug,
            output_format,
            epoch,
        } = self;
        let url = match epoch {
            Some(epoch) => addr
                .with_segments(&["v0", "stake", &epoch.to_string()])?
                .into_url(),
            _ => addr.with_segments(&["v0", "stake"])?.into_url(),
        };
        let builder = reqwest::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug).send()?;
        response.ok_response()?;
        let status = response.body().json_value()?;
        let formatted = output_format.format_json(status)?;
        println!("{}", formatted);
        Ok(())
    }
}
