use crate::jcli_lib::transaction::{common, Error};
use jormungandr_lib::interfaces::EvmTransaction;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AddEvmTransaction {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    /// hex encoded evm transaction
    pub certificate: EvmTransaction,
}

impl AddEvmTransaction {
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;
        self.common.store(&transaction)
    }
}
