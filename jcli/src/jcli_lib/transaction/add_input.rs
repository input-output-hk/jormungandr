use crate::jcli_lib::transaction::{common, Error};
use chain_impl_mockchain::{fragment::FragmentId, transaction::TransactionIndex};
use jormungandr_lib::interfaces;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AddInput {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    /// the Transaction ID which contains the credited funds to utilise.
    #[structopt(name = "TRANSACTION_ID")]
    pub transaction_id: FragmentId,

    /// the output index where the credited funds to utilise are.
    #[structopt(name = "INDEX")]
    pub index: TransactionIndex,

    /// the value
    #[structopt(name = "VALUE")]
    pub value: interfaces::Value,
}

impl AddInput {
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;

        transaction.add_input(interfaces::TransactionInput {
            input: interfaces::TransactionInputType::Utxo(self.transaction_id.into(), self.index),
            value: self.value,
        })?;

        self.common.store(&transaction)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use self::common::CommonTransaction;
    use super::*;
    use crate::jcli_lib::transaction::staging::Staging;
    use assert_fs::NamedTempFile;
    use chain_impl_mockchain::{key::Hash, value::Value};
    use std::str::FromStr;

    #[test]
    pub fn test_input_transaction_is_saved() {
        let tempfile = NamedTempFile::new("staging").unwrap();

        let transaction_id: FragmentId =
            Hash::from_str("c355a02d3b5337ad0e5f5940582675229f25bc03e7feebc3aa929738e1fec35e")
                .unwrap();
        let transaction_index: TransactionIndex = 0;
        let value: Value = Value(200);

        let staging = Staging::new();
        staging
            .store(&Some(tempfile.path()))
            .expect("cannot store staging file");

        let add_input = AddInput {
            common: CommonTransaction {
                staging_file: Some(tempfile.path().into()),
            },
            transaction_id,
            index: transaction_index,
            value: value.into(),
        };
        add_input
            .exec()
            .expect("error while executing AddInput action");

        let staging = Staging::load(&Some(tempfile.path())).unwrap();

        assert_eq!(
            staging.inputs().len(),
            1,
            "only one input should be created"
        );
        let input = &staging.inputs()[0];
        match input.input {
            interfaces::TransactionInputType::Account(_) => {
                panic!("didn't create an account input")
            }
            interfaces::TransactionInputType::Utxo(fid, index) => {
                assert_eq!(transaction_id.as_ref(), &fid, "fragment_id");
                assert_eq!(transaction_index, index, "fragment_index");
            }
        }
        assert_eq!(value, input.value.into(), "value");
    }
}
