use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Shutdown {
    /// Shutdown node
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
    },
}

impl Shutdown {
    pub fn exec(self) -> Result<(), Error> {
        let Shutdown::Get { addr, debug } = self;
        let url = addr.with_segments(&["v0", "shutdown"])?.into_url();
        let builder = reqwest::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug).send()?;
        response.ok_response()?;
        println!("Success");
        Ok(())
    }
}
