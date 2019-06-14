extern crate mktemp;
use crate::common::io;

extern crate chain_impl_mockchain;
use self::chain_impl_mockchain::key::Hash;
extern crate jcli;

use jcli::transaction::common::CommonTransaction;
use jcli::transaction::staging::Staging;
use jcli::transaction::*;
use std::str::FromStr;

#[test]
pub fn test_input_transaction_is_saved() {
    let temp_staging_file = io::get_path_in_temp("staging_file.tx").unwrap();
    let transaction_id: TransactionId =
        Hash::from_str("c355a02d3b5337ad0e5f5940582675229f25bc03e7feebc3aa929738e1fec35e").unwrap();
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

    assert_eq!(
        staging.inputs().len(),
        1,
        "only one input should be created"
    );
    let input = &staging.inputs()[0];
    assert_eq!(transaction_id.as_ref(), &input.input_ptr, "transaction_id");
    assert_eq!(
        transaction_index, input.index_or_account,
        "transaction_index"
    );
    assert_eq!(value, input.value, "value");
}
