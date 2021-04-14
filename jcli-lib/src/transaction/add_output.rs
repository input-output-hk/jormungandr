use crate::transaction::{common, Error};
use chain_impl_mockchain::transaction::Output;
use jormungandr_lib::interfaces;
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub struct AddOutput {
    #[cfg_attr(feature = "structopt", structopt(flatten))]
    pub common: common::CommonTransaction,

    /// the UTxO address or account address to credit funds to
    #[cfg_attr(feature = "structopt", structopt(name = "ADDRESS"))]
    pub address: interfaces::Address,

    /// the value
    #[cfg_attr(feature = "structopt", structopt(name = "VALUE"))]
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
