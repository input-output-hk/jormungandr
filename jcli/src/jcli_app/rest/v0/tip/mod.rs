use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, RestApiSender, TlsCert};
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
        #[structopt(flatten)]
        tls: TlsCert,
    },
}

impl Tip {
    pub fn exec(self) -> Result<(), Error> {
        let (addr, debug, tls) = match self {
            Tip::Get { addr, debug, tls } => (addr, debug, tls),
        };
        let url = addr.with_segments(&["v0", "tip"])?.into_url();
        let builder = reqwest::blocking::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug, &tls).send()?;
        response.ok_response()?;
        let tip = response.body().text();
        println!("{}", tip.as_ref());
        Ok(())
    }
}
