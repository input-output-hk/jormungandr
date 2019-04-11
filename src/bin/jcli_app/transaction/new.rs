use chain_impl_mockchain::transaction::{NoExtra, Transaction};
use structopt::StructOpt;

use jcli_app::transaction::common;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct New {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,
}

custom_error! {pub NewError
    WriteTransaction { source: common::CommonError } = "cannot create new transaction"
}

impl New {
    pub fn exec(self) -> Result<(), NewError> {
        Ok(self.common.write_transaction(&Transaction {
            inputs: Vec::new(),
            outputs: Vec::new(),
            extra: NoExtra,
        })?)
    }
}
