use crate::jcli_app::transaction::{common, Error};
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

        let fee_algo = self.fee.linear_fee();
        let output_policy = match self.change {
            None => OutputPolicy::Forget,
            Some(change) => OutputPolicy::One(change.into()),
        };

        let _balance = transaction.balance_inputs_outputs(&fee_algo, output_policy)?;

        self.common.store(&transaction)?;
        Ok(())
    }
}
