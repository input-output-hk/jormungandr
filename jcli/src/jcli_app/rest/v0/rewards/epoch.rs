use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, RestApiSender, TlsCert};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Epoch {
    /// Get rewards for epoch
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        /// Epoch number
        epoch: u32,
        #[structopt(flatten)]
        tls: TlsCert,
    },
}

impl Epoch {
    pub fn exec(self) -> Result<(), Error> {
        let Epoch::Get {
            addr,
            debug,
            epoch,
            tls,
        } = self;
        let url = addr
            .with_segments(&["v0", "rewards", "epoch", &epoch.to_string()])?
            .into_url();
        let builder = reqwest::blocking::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug, &tls).send()?;
        response.ok_response()?;
        let epoch = response.body().text();
        println!("{}", epoch.as_ref());
        Ok(())
    }
}
