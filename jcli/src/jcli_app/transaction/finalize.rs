use chain_addr::Address;
use chain_impl_mockchain::txbuilder::OutputPolicy;
use jcli_app::transaction::{common, Error};
use jormungandr_utils::structopt;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Finalize {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    #[structopt(flatten)]
    pub fee: common::CommonFees,

    /// Set the change in the given address
    #[structopt(parse(try_from_str = "structopt::try_parse_address"))]
    pub change: Option<Address>,
}

impl Finalize {
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;

        let fee_algo = self.fee.linear_fee();
        let output_policy = match self.change {
            None => OutputPolicy::Forget,
            Some(change) => OutputPolicy::One(change),
        };

        let _balance = transaction.finalize(fee_algo, output_policy)?;

        self.common.store(&transaction)?;
        Ok(())
    }
}
