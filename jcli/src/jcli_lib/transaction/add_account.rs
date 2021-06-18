use crate::jcli_lib::transaction::{common, Error};
use chain_addr::{Address, Kind};
use chain_impl_mockchain::transaction::UnspecifiedAccountIdentifier;
use jormungandr_lib::interfaces;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AddAccount {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    /// the account to debit the funds from
    #[structopt(name = "ACCOUNT")]
    pub account: interfaces::Address,

    /// the value
    #[structopt(name = "VALUE")]
    pub value: interfaces::Value,
}

impl AddAccount {
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;

        let account_id = match Address::from(self.account).kind() {
            Kind::Account(key) => {
                UnspecifiedAccountIdentifier::from_single_account(key.clone().into())
            }
            Kind::Multisig(key) => UnspecifiedAccountIdentifier::from_multi_account((*key).into()),
            Kind::Single(_) => return Err(Error::AccountAddressSingle),
            Kind::Group(_, _) => return Err(Error::AccountAddressGroup),
            Kind::Script(_) => return Err(Error::AccountAddressScript),
        };

        transaction.add_input(interfaces::TransactionInput {
            input: interfaces::TransactionInputType::Account(account_id.into()),
            value: self.value,
        })?;

        self.common.store(&transaction)
    }
}
