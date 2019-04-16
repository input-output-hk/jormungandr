use chain_addr::Address;
use chain_impl_mockchain::{transaction::Output, value::Value};
use jcli_app::transaction::{common, staging::StagingError};
use jormungandr_utils::structopt;
use structopt::StructOpt;

pub type AddOutputError = StagingError;

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
        let mut transaction = self.common.load()?;

        transaction.add_output(Output {
            address: self.address,
            value: self.value,
        })?;

        self.common.store(&transaction)
    }
}
