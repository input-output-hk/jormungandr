use jormungandr_lib::crypto::hash::Hash;
use jormungandr_testing_utils::testing::jcli::JCli;

lazy_static! {
    static ref FAKE_INPUT_TRANSACTION_ID: Hash = {
        "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193"
            .parse()
            .unwrap()
    };
}
const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";

#[test]
pub fn test_cannot_create_input_with_negative_amount() {
    let jcli: JCli = Default::default();
    jcli.transaction_builder(Hash::from_hex(FAKE_GENESIS_HASH).unwrap())
        .new_transaction()
        .add_input_expect_fail(
            &FAKE_INPUT_TRANSACTION_ID,
            0,
            "-100",
            "Found argument '-1' which wasn't expected",
        );
}

#[test]
pub fn test_cannot_create_input_with_too_big_utxo_amount() {
    let jcli: JCli = Default::default();
    jcli.transaction_builder(Hash::from_hex(FAKE_GENESIS_HASH).unwrap())
        .new_transaction()
        .add_input_expect_fail(
            &FAKE_INPUT_TRANSACTION_ID,
            0,
            "100000000000000000000",
            "error: Invalid value for '<VALUE>': number too large to fit in target type",
        );
}

#[test]
#[cfg(not(target_os = "linux"))]
pub fn test_cannot_create_input_when_staging_file_is_readonly() {
    use jortestkit::file;
    let jcli: JCli = Default::default();
    let mut transaction_wrapper =
        jcli.transaction_builder(Hash::from_hex(FAKE_GENESIS_HASH).unwrap());

    transaction_wrapper.new_transaction();
    file::make_readonly(&transaction_wrapper.staging_file_path());
    transaction_wrapper.add_input_expect_fail(&FAKE_INPUT_TRANSACTION_ID, 0, "100", "denied");
}
