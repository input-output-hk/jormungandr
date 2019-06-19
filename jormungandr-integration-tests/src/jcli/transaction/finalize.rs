use crate::common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use crate::common::startup;
use jormungandr_lib::crypto::hash::Hash;

const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";

lazy_static! {
    static ref FAKE_INPUT_TRANSACTION_ID: Hash = {
        "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193"
            .parse()
            .unwrap()
    };
}

#[test]
pub fn test_unbalanced_output_utxo_transaction_is_not_finalized() {
    let receiver = startup::create_new_utxo_address();

    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH)
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, 0, &100.into())
        .assert_add_output(&receiver.address, &150.into())
        .assert_finalize_fail("not enough input for making transaction");
}
