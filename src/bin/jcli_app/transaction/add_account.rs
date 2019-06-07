use chain_addr::{Address, Kind};
use chain_impl_mockchain::{
    transaction::{AccountIdentifier, Input, InputEnum},
    value::Value,
};
use structopt::StructOpt;

use jcli_app::transaction::{common, Error};
use jormungandr_utils::structopt;

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
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;

        let account_id = match self.account.kind() {
            Kind::Account(key) => AccountIdentifier::from_single_account(key.clone().into()),
            Kind::Single(_) => return Err(Error::AccountAddressSingle),
            Kind::Group(_, _) => return Err(Error::AccountAddressGroup),
            Kind::Multisig(_) => return Err(Error::AccountAddressMultisig),
        };

        transaction.add_input(Input::from_enum(InputEnum::AccountInput(
            account_id, self.value,
        )))?;

        self.common.store(&transaction)
    }
}
