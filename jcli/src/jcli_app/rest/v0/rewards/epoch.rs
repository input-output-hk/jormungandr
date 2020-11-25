use crate::jcli_app::rest::{Error, RestArgs};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Epoch {
    /// Get rewards for epoch
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        /// Epoch number
        epoch: u32,
    },
}

impl Epoch {
    pub fn exec(self) -> Result<(), Error> {
        let Epoch::Get { args, epoch } = self;
        let response = args
            .request_with_args(
                &["v0", "rewards", "epoch", &epoch.to_string()],
                |client, url| client.get(url),
            )?
            .text()?;
        println!("{}", response);
        Ok(())
    }
}
