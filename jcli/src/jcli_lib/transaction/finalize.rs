use crate::{
    jcli_lib::transaction::{common, Error},
    transaction::staging::Staging,
};
use chain_impl_mockchain::transaction::OutputPolicy;
use jormungandr_lib::interfaces;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Finalize {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    #[structopt(flatten)]
    pub fee: common::CommonFees,

    /// Set the change in the given address
    pub change: Option<interfaces::Address>,
}

impl Finalize {
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;

        finalize(self.fee, self.change, &mut transaction)?;

        self.common.store(&transaction)?;
        Ok(())
    }
}

pub fn finalize(
    fee: common::CommonFees,
    change: Option<interfaces::Address>,
    transaction: &mut Staging,
) -> Result<(), Error> {
    let fee_algo = fee.linear_fee();
    let output_policy = match change {
        None => OutputPolicy::Forget,
        Some(change) => OutputPolicy::One(change.into()),
    };
    let _balance = transaction.balance_inputs_outputs(&fee_algo, output_policy)?;
    Ok(())
}
