use chain_impl_mockchain::transaction::Output;
use jcli_app::transaction::{common, Error};
use jormungandr_lib::interfaces;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AddOutput {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    /// the UTxO address or account address to credit funds to
    #[structopt(name = "ADDRESS")]
    pub address: interfaces::Address,

    /// the value
    #[structopt(name = "VALUE")]
    pub value: interfaces::Value,
}

impl AddOutput {
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;

        transaction.add_output(Output {
            address: self.address.into(),
            value: self.value.into(),
        })?;

        self.common.store(&transaction)
    }
}
