use chain_addr::{Address, Kind};
use chain_impl_mockchain::{
    transaction::{Input, InputEnum},
    value::Value,
};
use structopt::StructOpt;

use jcli_app::transaction::common;
use jormungandr_utils::structopt;

custom_error! {pub AddAccountError
    ReadTransaction { error: common::CommonError } = "cannot read the transaction: {error}",
    WriteTransaction { error: common::CommonError } = "cannot save changes of the transaction: {error}",
    InvalidAddressSingle = "Invalid input account, this is a UTxO address.",
    InvalidAddressGroup = "Invalid input account, this is a UTxO address with delegation.",
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AddAccount {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    /// the account to debit the funds from
    #[structopt(name = "ACCOUNT", parse(try_from_str = "structopt::try_parse_address"))]
    pub account: Address,

    /// the value
    #[structopt(name = "VALUE", parse(try_from_str = "structopt::try_parse_value"))]
    pub value: Value,
}

impl AddAccount {
    pub fn exec(self) -> Result<(), AddAccountError> {
        let mut transaction = self
            .common
            .load_transaction()
            .map_err(|error| AddAccountError::ReadTransaction { error })?;

        let account_identifier = match self.account.kind() {
            Kind::Account(key) => key.clone().into(),
            Kind::Single(_) => return Err(AddAccountError::InvalidAddressSingle),
            Kind::Group(_, _) => return Err(AddAccountError::InvalidAddressGroup),
        };

        transaction
            .inputs
            .push(Input::from_enum(InputEnum::AccountInput(
                account_identifier,
                self.value,
            )));

        Ok(self
            .common
            .write_transaction(&transaction)
            .map_err(|error| AddAccountError::WriteTransaction { error })?)
    }
}
