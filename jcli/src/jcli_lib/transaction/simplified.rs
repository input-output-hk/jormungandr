use crate::jcli_lib::transaction::{common, Error};
use crate::transaction;
use crate::transaction::staging::Staging;
use chain_impl_mockchain::transaction::Output;
use jormungandr_lib::interfaces;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Simplified {
    input_address_sk: String,

    /// the account to debit the funds from
    #[structopt(name = "ACCOUNT")]
    pub faucet_address: interfaces::Address,

    /// the UTxO address or account address to credit funds to
    #[structopt(name = "ADDRESS")]
    pub receiver_address: interfaces::Address,

    /// the value
    #[structopt(name = "VALUE")]
    pub value: interfaces::Value,

    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    #[structopt(flatten)]
    pub fee: common::CommonFees,

    /// Set the change in the given address
    pub change: Option<interfaces::Address>,
}

impl Simplified {
    pub fn exec(self) -> Result<(), Error> {
        simplified_transaction(
            self.faucet_address,
            self.receiver_address,
            self.value,
            self.fee,
            self.change,
        )?;
        Ok(())
    }
}

pub fn simplified_transaction(
    faucet_address: interfaces::Address,
    receiver_address: interfaces::Address,
    value: interfaces::Value,
    fee: common::CommonFees,
    change: Option<interfaces::Address>,
) -> Result<(), Error> {
    let mut transaction = Staging::new();

    // add account
    transaction::add_account::add_account(faucet_address, value.clone(), &mut transaction)?;

    // add output
    transaction.add_output(Output {
        address: receiver_address.into(),
        value: value.into(),
    })?;

    //finalize
    transaction::finalize::finalize(fee, change, &mut transaction)?;

    Ok(())
}
