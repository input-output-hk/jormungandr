use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Diagnostic {
    /// Get system diagnostic information
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
    },
}

impl Diagnostic {
    pub fn exec(self) -> Result<(), Error> {
        let (addr, debug) = match self {
            Diagnostic::Get { addr, debug } => (addr, debug),
        };
        let url = addr.with_segments(&["v0", "diagnostic"])?.into_url();
        let builder = reqwest::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug).send()?;
        response.ok_response()?;
        let diagnostic = response.body().text();
        println!("{}", diagnostic.as_ref());
        Ok(())
    }
}
