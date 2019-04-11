use chain_addr::Address;
use chain_impl_mockchain::{transaction::Output, value::Value};
use jcli_app::transaction::common;
use jormungandr_utils::structopt;
use structopt::StructOpt;

custom_error! {pub AddOutputError
    ReadTransaction { error: common::CommonError } = "cannot read the transaction: {error}",
    WriteTransaction { error: common::CommonError } = "cannot save changes of the transaction: {error}",
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AddOutput {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    /// the UTxO address or account address to credit funds to
    #[structopt(name = "ADDRESS", parse(try_from_str = "structopt::try_parse_address"))]
    pub address: Address,

    /// the value
    #[structopt(name = "VALUE", parse(try_from_str = "structopt::try_parse_value"))]
    pub value: Value,
}

impl AddOutput {
    pub fn exec(self) -> Result<(), AddOutputError> {
        let mut transaction = self
            .common
            .load_transaction()
            .map_err(|error| AddOutputError::ReadTransaction { error })?;

        transaction.outputs.push(Output {
            address: self.address,
            value: self.value,
        });

        Ok(self
            .common
            .write_transaction(&transaction)
            .map_err(|error| AddOutputError::WriteTransaction { error })?)
    }
}
