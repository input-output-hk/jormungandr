use crate::jcli_app::rest::{Error, RestArgs};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Tip {
    /// Get tip ID
    Get {
        #[structopt(flatten)]
        args: RestArgs,
    },
}

impl Tip {
    pub fn exec(self) -> Result<(), Error> {
        let args = match self {
            Tip::Get { args } => args,
        };
        let response = args
            .request_with_args(&["v0", "tip"], |client, url| client.get(url))?
            .text()?;
        println!("{}", response);
        Ok(())
    }
}
