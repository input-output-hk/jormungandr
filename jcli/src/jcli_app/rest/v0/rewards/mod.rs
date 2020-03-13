use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, OutputFormat, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "rewards", rename_all = "kebab-case")]
pub enum Rewards {
    /// Rewards distribution history one or more epochs starting from the last one
    History(History),
    /// Rewards distribution for a specific epoch
    Epoch(Epoch),
}

impl Rewards {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Rewards::History(history) => history.exec(),
            Rewards::Epoch(epoch) => epoch.exec(),
        }
    }
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum History {
    /// Get rewards for one or more epochs
    Get {
        #[structopt(flatten)]
        common: Common,
        /// Number of epochs
        length: usize,
    },
}

impl History {
    pub fn exec(self) -> Result<(), Error> {
        let History::Get { common, length } = self;
        let url = common
            .addr
            .with_segments(&["v0", "rewards", "history", &length.to_string()])?
            .into_url();
        let builder = reqwest::Client::new().get(url);
        let response = RestApiSender::new(builder, &common.debug).send()?;
        response.ok_response()?;
        let state = response.body().json_value()?;
        let formatted = common.output_format.format_json(state)?;
        println!("{}", formatted);
        Ok(())
    }
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Epoch {
    /// Get rewards for epoch
    Get {
        #[structopt(flatten)]
        common: Common,
        /// Epoch number
        epoch: u32,
    },
}

impl Epoch {
    pub fn exec(self) -> Result<(), Error> {
        let Epoch::Get { common, epoch } = self;
        let url = common
            .addr
            .with_segments(&["v0", "rewards", "epoch", &epoch.to_string()])?
            .into_url();
        let builder = reqwest::Client::new().get(url);
        let response = RestApiSender::new(builder, &common.debug).send()?;
        response.ok_response()?;
        let state = response.body().json_value()?;
        let formatted = common.output_format.format_json(state)?;
        println!("{}", formatted);
        Ok(())
    }
}

#[derive(StructOpt)]
pub struct Common {
    #[structopt(flatten)]
    addr: HostAddr,
    #[structopt(flatten)]
    debug: DebugFlag,
    #[structopt(flatten)]
    output_format: OutputFormat,
}
