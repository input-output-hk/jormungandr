const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
use jormungandr_testing_utils::testing::{jcli_wrapper::JCLITransactionWrapper, startup};

#[test]
pub fn test_add_account_for_utxo_delegation_address_fails() {
    let sender = startup::create_new_delegation_address();

    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH).assert_add_account_fail(
        &sender.address,
        &100,
        "Invalid input account, this is a UTxO address",
    );
}
