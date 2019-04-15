use structopt::StructOpt;

use jcli_app::transaction::{
    common,
    staging::{Staging, StagingError},
};

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct New {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,
}

custom_error! {pub NewError
    WriteTransaction { source: StagingError } = "cannot create new transaction"
}

impl New {
    pub fn exec(self) -> Result<(), NewError> {
        let staging = Staging::new();
        Ok(self.common.store(&staging)?)
    }
}
