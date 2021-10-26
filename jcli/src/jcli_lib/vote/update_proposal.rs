use crate::jcli_lib::vote::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct UpdateProposal {}

impl UpdateProposal {
    pub fn exec(&self) -> Result<(), Error> {
        Ok(())
    }
}
