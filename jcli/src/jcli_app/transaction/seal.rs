use crate::jcli_app::transaction::{common, Error};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Seal {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,
}

impl Seal {
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;
        transaction.seal()?;
        self.common.store(&transaction)
    }
}
