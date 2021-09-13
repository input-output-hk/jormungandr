mod vote;

use crate::jcli_lib::rest::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum V1 {
    Vote(vote::Vote),
}

impl V1 {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            V1::Vote(vote) => vote.exec(),
        }
    }
}
