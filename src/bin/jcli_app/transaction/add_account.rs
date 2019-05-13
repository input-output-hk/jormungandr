use chain_addr::{Address, Kind};
use chain_impl_mockchain::{
    transaction::{AccountIdentifier, Input, InputEnum},
    value::Value,
};
use structopt::StructOpt;

use jcli_app::transaction::{common, staging::StagingError};
use jormungandr_utils::structopt;

custom_error! {pub AddAccountError
    ReadTransaction { error: StagingError } = "cannot read the transaction: {error}",
    WriteTransaction { error: StagingError } = "cannot save changes of the transaction: {error}",
    AddInput { source: StagingError } = "cannot add account",
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
            .load()
            .map_err(|error| AddAccountError::ReadTransaction { error })?;

        let account_identifier = match self.account.kind() {
            Kind::Account(key) => AccountIdentifier::from_single_account(key.clone().into()),
            Kind::Single(_) => return Err(AddAccountError::InvalidAddressSingle),
            Kind::Group(_, _) => return Err(AddAccountError::InvalidAddressGroup),
            Kind::Multisig(_) => unimplemented!(),
        };

        transaction.add_input(Input::from_enum(InputEnum::AccountInput(
            account_identifier,
            self.value,
        )))?;

        Ok(self
            .common
            .store(&transaction)
            .map_err(|error| AddAccountError::WriteTransaction { error })?)
    }
}
