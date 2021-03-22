use crate::jcli_lib::rest::{Error, RestArgs};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum History {
    /// Get rewards for one or more epochs
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        /// Number of epochs
        length: usize,
    },
}

impl History {
    pub fn exec(self) -> Result<(), Error> {
        let History::Get { args, length } = self;
        let response = args
            .client()?
            .get(&["v0", "rewards", "history", &length.to_string()])
            .execute()?
            .text()?;
        println!("{}", response);
        Ok(())
    }
}
