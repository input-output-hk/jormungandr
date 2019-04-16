use chain_addr::Address;
use chain_impl_mockchain::txbuilder::OutputPolicy;
use jcli_app::transaction::{common, staging::StagingError};
use jormungandr_utils::structopt;
use structopt::StructOpt;

custom_error! {pub FinalizeError
    ReadTransaction { error: StagingError } = "cannot read the transaction: {error}",
    WriteTransaction { error: StagingError } = "cannot save changes of the transaction: {error}",
    TransactionCannotBeFinalizeed { source: StagingError } = "Transaction cannot be finalized"
}

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
    pub fn exec(self) -> Result<(), FinalizeError> {
        let mut transaction = self
            .common
            .load()
            .map_err(|error| FinalizeError::ReadTransaction { error })?;

        let fee_algo = self.fee.linear_fee();
        let output_policy = match self.change {
            None => OutputPolicy::Forget,
            Some(change) => OutputPolicy::One(change),
        };

        let _balance = transaction.finalize(fee_algo, output_policy)?;

        Ok(self
            .common
            .store(&transaction)
            .map_err(|error| FinalizeError::WriteTransaction { error })?)
    }
}
