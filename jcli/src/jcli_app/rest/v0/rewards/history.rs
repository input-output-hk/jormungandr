use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, RestApiSender, TlsCert};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum History {
    /// Get rewards for one or more epochs
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        /// Number of epochs
        length: usize,
        #[structopt(flatten)]
        tls: TlsCert,
    },
}

impl History {
    pub fn exec(self) -> Result<(), Error> {
        let History::Get {
            addr,
            debug,
            length,
            tls,
        } = self;
        let url = addr
            .with_segments(&["v0", "rewards", "history", &length.to_string()])?
            .into_url();
        let builder = reqwest::blocking::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug, &tls).send()?;
        response.ok_response()?;
        let history = response.body().text();
        println!("{}", history.as_ref());
        Ok(())
    }
}
