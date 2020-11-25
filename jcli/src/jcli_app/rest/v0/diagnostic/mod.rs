use crate::jcli_app::rest::{Error, RestArgs};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Diagnostic {
    /// Get system diagnostic information
    Get {
        #[structopt(flatten)]
        args: RestArgs,
    },
}

impl Diagnostic {
    pub fn exec(self) -> Result<(), Error> {
        let args = match self {
            Diagnostic::Get { args } => args,
        };
        let response =
            args.request_text_with_args(&["v0", "diagnostic"], |client, url| client.get(url))?;
        println!("{}", response);
        Ok(())
    }
}
