use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, RestApiSender, TlsCert};
use structopt::StructOpt;

/// Shutdown node
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Shutdown {
    Post {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        #[structopt(flatten)]
        tls: TlsCert,
    },
}

impl Shutdown {
    pub fn exec(self) -> Result<(), Error> {
        let Shutdown::Post { addr, debug, tls } = self;
        let url = addr.with_segments(&["v0", "shutdown"])?.into_url();
        let builder = reqwest::blocking::Client::new().post(url);
        let response = RestApiSender::new(builder, &debug, &tls).send()?;
        response.ok_response()?;
        println!("Success");
        Ok(())
    }
}
