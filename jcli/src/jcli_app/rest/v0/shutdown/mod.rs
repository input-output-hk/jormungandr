use crate::jcli_app::rest::{Error, RestArgs};
use structopt::StructOpt;

/// Shutdown node
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Shutdown {
    Post {
        #[structopt(flatten)]
        args: RestArgs,
    },
}

impl Shutdown {
    pub fn exec(self) -> Result<(), Error> {
        let Shutdown::Post { args } = self;
        args.request_with_args(&["v0", "shutdown"], |client, url| client.post(url))?;
        println!("Success");
        Ok(())
    }
}
