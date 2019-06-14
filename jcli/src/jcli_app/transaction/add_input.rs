use chain_impl_mockchain::{
    transaction::{Input, InputEnum, TransactionId, TransactionIndex, UtxoPointer},
    value::Value,
};
use jcli_app::transaction::{common, Error};
use jormungandr_utils::structopt;
use structopt::StructOpt;

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
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;

        transaction.add_input(Input::from_enum(InputEnum::UtxoInput(UtxoPointer {
            transaction_id: self.transaction_id,
            output_index: self.index,
            value: self.value,
        })))?;

        self.common.store(&transaction)?;
        Ok(())
    }
}
