use structopt::StructOpt;

use jcli_app::transaction::{common, staging::Staging, Error};

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct New {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,
}

impl New {
    pub fn exec(self) -> Result<(), Error> {
        let staging = Staging::new();
        self.common.store(&staging)
    }
}
