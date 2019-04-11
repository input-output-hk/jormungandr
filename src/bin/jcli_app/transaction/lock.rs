use chain_addr::Address;
use chain_impl_mockchain::{
    transaction::AuthenticatedTransaction,
    txbuilder::{OutputPolicy, TransactionBuilder},
};
use jcli_app::transaction::common;
use jormungandr_utils::structopt;
use structopt::StructOpt;

custom_error! {pub LockError
    ReadTransaction { error: common::CommonError } = "cannot read the transaction: {error}",
    WriteTransaction { error: common::CommonError } = "cannot save changes of the transaction: {error}",
    TransactionCannotBeLocked { source : chain_impl_mockchain::txbuilder::Error } = "Transaction cannot be finalized"
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Lock {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    #[structopt(flatten)]
    pub fee: common::CommonFees,

    /// Set the change in the given address
    #[structopt(parse(try_from_str = "structopt::try_parse_address"))]
    pub change: Option<Address>,
}

impl Lock {
    pub fn exec(self) -> Result<(), LockError> {
        let transaction = self
            .common
            .load_transaction()
            .map_err(|error| LockError::ReadTransaction { error })?;

        let builder = TransactionBuilder::from(transaction);
        let fee_algo = self.fee.linear_fee();
        let output_policy = match self.change {
            None => OutputPolicy::Forget,
            Some(change) => OutputPolicy::One(change),
        };

        let (_balance, finalized) = builder.finalize(fee_algo, output_policy)?;

        let auth = AuthenticatedTransaction {
            transaction: finalized,
            witnesses: Vec::new(),
        };

        Ok(self
            .common
            .write_auth_transaction(&auth)
            .map_err(|error| LockError::WriteTransaction { error })?)
    }
}
