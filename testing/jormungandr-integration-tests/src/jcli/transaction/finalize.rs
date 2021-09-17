use jormungandr_lib::{crypto::hash::Hash, interfaces::BlockDate};
use jormungandr_testing_utils::testing::common::{jcli::JCli, startup};

lazy_static! {
    static ref FAKE_INPUT_TRANSACTION_ID: Hash = {
        "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193"
            .parse()
            .unwrap()
    };
    static ref FAKE_GENESIS_HASH: Hash = {
        "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193"
            .parse()
            .unwrap()
    };
}

#[test]
pub fn test_unbalanced_output_utxo_transaction_is_not_finalized() {
    let receiver = startup::create_new_utxo_address();
    let jcli: JCli = Default::default();

    jcli.transaction_builder(*FAKE_GENESIS_HASH)
        .new_transaction()
        .add_input(&FAKE_INPUT_TRANSACTION_ID, 0, "100")
        .add_output(&receiver.address().to_string(), 150.into())
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize_expect_fail("not enough input for making transaction");
}
