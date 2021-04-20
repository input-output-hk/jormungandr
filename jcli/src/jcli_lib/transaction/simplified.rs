use crate::transaction;
use crate::transaction::common;
use crate::transaction::staging::Staging;
use chain_impl_mockchain::transaction::Output;
use jormungandr_lib::interfaces;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Send {
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

impl Send {
    pub fn exec(self) -> std::io::Result<()> {
        let mut transaction = Staging::new();

        // add account
        transaction::add_account::add_account(
            self.faucet_address,
            self.value.clone(),
            &mut transaction,
        )?;

        // add output
        transaction.add_output(Output {
            address: self.receiver_address.into(),
            value: self.value.into(),
        })?;

        //finalize
        transaction::finalize::finalize(self.fee, self.change, &mut transaction)?;

        Ok(())
    }
}
