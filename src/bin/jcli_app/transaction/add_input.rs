use chain_impl_mockchain::{
    transaction::{Input, InputEnum, TransactionId, TransactionIndex, UtxoPointer},
    value::Value,
};
use jcli_app::transaction::common;
use jormungandr_utils::structopt;
use structopt::StructOpt;

custom_error! {pub AddInputError
    ReadTransaction { error: common::CommonError } = "cannot read the transaction: {error}",
    WriteTransaction { error: common::CommonError } = "cannot save changes of the transaction: {error}",
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AddInput {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    /// the Transaction ID which contains the credited funds to utilise.
    #[structopt(name = "TRANSACTION_ID")]
    pub transaction_id: TransactionId,

    /// the output index where the credited funds to utilise are.
    #[structopt(name = "INDEX")]
    pub index: TransactionIndex,

    /// the value
    #[structopt(name = "VALUE", parse(try_from_str = "structopt::try_parse_value"))]
    pub value: Value,
}

impl AddInput {
    pub fn exec(self) -> Result<(), AddInputError> {
        let mut transaction = self
            .common
            .load_transaction()
            .map_err(|error| AddInputError::ReadTransaction { error })?;

        transaction
            .inputs
            .push(Input::from_enum(InputEnum::UtxoInput(UtxoPointer {
                transaction_id: self.transaction_id,
                output_index: self.index,
                value: self.value,
            })));

        Ok(self
            .common
            .write_transaction(&transaction)
            .map_err(|error| AddInputError::WriteTransaction { error })?)
    }
}
