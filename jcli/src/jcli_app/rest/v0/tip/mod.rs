use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Tip {
    /// Get tip ID
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
    },
}

impl Tip {
    pub fn exec(self) -> Result<(), Error> {
        let (addr, debug) = match self {
            Tip::Get { addr, debug } => (addr, debug),
        };
        let url = addr.with_segments(&["v0", "tip"])?.into_url();
        let builder = reqwest::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug).send()?;
        response.ok_response()?;
        let tip = response.body().text();
        println!("{}", tip.as_ref());
        Ok(())
    }
}
