use chain_impl_mockchain::{
    transaction::{Input, InputEnum, TransactionId, TransactionIndex, UtxoPointer},
    value::Value,
};
use jcli_app::transaction::{common, staging::StagingError};
use jormungandr_utils::structopt;
use structopt::StructOpt;

pub type AddInputError = StagingError;

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
        let mut transaction = self.common.load()?;

        transaction.add_input(Input::from_enum(InputEnum::UtxoInput(UtxoPointer {
            transaction_id: self.transaction_id,
            output_index: self.index,
            value: self.value,
        })))?;

        self.common.store(&transaction)
    }
}

#[cfg(test)]
mod tests {

    extern crate mktemp;
    use self::common::CommonTransaction;
    use super::*;
    use crate::jcli_app::transaction::staging::Staging;
    use crate::jcli_app::utils::io;
    use chain_impl_mockchain::key::Hash;
    use std::str::FromStr;

    #[test]
    pub fn test_input_transaction_is_saved() {
        let temp_staging_file = io::get_path_in_temp("staging_file.tx").unwrap();
        let transaction_id: TransactionId =
            Hash::from_str("c355a02d3b5337ad0e5f5940582675229f25bc03e7feebc3aa929738e1fec35e")
                .unwrap();
        let transaction_index: TransactionIndex = 0;
        let value: Value = Value(200);

        let staging = Staging::new();
        staging
            .store(&Some(&temp_staging_file))
            .expect("cannot store staging file");

        let add_input = AddInput {
            common: CommonTransaction {
                staging_file: Some(temp_staging_file.clone()),
            },
            transaction_id: transaction_id,
            index: transaction_index,
            value: value,
        };
        add_input
            .exec()
            .expect("error while executing AddInput action");

        let staging = Staging::load(&Some(&temp_staging_file)).unwrap();

        assert_eq!(staging.inputs().len(), 1, "only one input should be created");
        let input = &staging.inputs()[0];
        assert_eq!(transaction_id.as_ref(), &input.input_ptr, "transaction_id");
        assert_eq!(
            transaction_index, input.index_or_account,
            "transaction_index"
        );
        assert_eq!(value, input.value, "value");
    }
}
