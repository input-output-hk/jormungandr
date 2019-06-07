use crate::common::data::witness::Witness;
use crate::common::file_utils;
use crate::common::jcli_wrapper;
use crate::common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use crate::common::startup;
use std::path::PathBuf;

const FAKE_INPUT_TRANSACTION_ID: &str =
    "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_ACCOUNT_ADDRESS: &str = "ta1s5fzaewqt7yq89vma8atryks9vyvfacr5hq8wm64vhn36f62lyvrzuxm7dm";

#[test]
pub fn test_utxo_transation_with_more_than_one_witness_per_input_is_rejected() {
    let reciever = startup::create_new_utxo_address();

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize();

    let witness1 = transaction_wrapper.create_witness_default("utxo");
    let witness2 = transaction_wrapper.create_witness_default("utxo");

    transaction_wrapper
        .assert_make_witness(&witness1)
        .assert_add_witness(&witness1)
        .assert_make_witness(&witness2)
        .assert_add_witness_fail(&witness2, "cannot add anymore witnesses");
}

#[test]
pub fn test_utxo_transation_with_dupicated_witness_is_rejected() {
    let reciever = startup::create_new_utxo_address();

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize();

    let witness1 = transaction_wrapper.create_witness_default("utxo");

    transaction_wrapper
        .assert_make_witness(&witness1)
        .assert_add_witness(&witness1)
        .assert_add_witness_fail(&witness1, "cannot add anymore witnesses");
}

#[test]
pub fn test_utxo_transation_with_address_type_witness_is_rejected() {
    let reciever = startup::create_new_utxo_address();

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let witness = transaction_wrapper.create_witness_default("account");

    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .seal_with_witness(&witness)
        .assert_transaction_to_message_fails("cannot seal: Invalid witness type at index 0");
}

#[test]
pub fn test_account_transation_with_utxo_type_witness_is_rejected() {
    let reciever = startup::create_new_account_address();
    let sender = startup::create_new_account_address();

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let witness = transaction_wrapper.create_witness_default("utxo");
    transaction_wrapper
        .assert_add_account(&sender.address, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .seal_with_witness(&witness)
        .assert_transaction_to_message_fails("cannot seal: Invalid witness type at index 0");
}

#[test]
pub fn test_make_witness_with_unknown_type_fails() {
    let reciever = startup::create_new_utxo_address();

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let witness = transaction_wrapper.create_witness_default("Unknown");
    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_make_witness_fails(&witness, "Invalid witness type");
}

#[test]
pub fn test_make_witness_with_invalid_private_key_fails() {
    let reciever = startup::create_new_utxo_address();

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);

    let mut private_key = jcli_wrapper::assert_key_generate_default();
    private_key.push('3');

    let witness = Witness::new(
        FAKE_GENESIS_HASH,
        FAKE_INPUT_TRANSACTION_ID,
        "utxo",
        &private_key,
        &0,
    );

    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_make_witness_fails(&witness, "Invalid Bech32");
}

#[test]
pub fn test_make_witness_with_non_existing_private_key_file_fails() {
    let reciever = startup::create_new_utxo_address();
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let private_key = jcli_wrapper::assert_key_generate_default();

    let mut witness = Witness::new(
        FAKE_GENESIS_HASH,
        FAKE_INPUT_TRANSACTION_ID,
        "utxo",
        &private_key,
        &0,
    );
    witness.private_key_path = PathBuf::from("a");
    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_make_witness_fails(&witness, "NotFound");
}

#[test]
#[cfg(not(target_os = "linux"))]
pub fn test_make_witness_with_readonly_private_key_file_fails() {
    let reciever = startup::create_new_utxo_address();
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let private_key = jcli_wrapper::assert_key_generate_default();

    let witness = Witness::new(
        FAKE_GENESIS_HASH,
        FAKE_INPUT_TRANSACTION_ID,
        "utxo",
        &private_key,
        &0,
    );
    file_utils::make_readonly(&witness.file);
    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_make_witness_fails(&witness, "denied");
}

#[test]
pub fn test_make_witness_with_wrong_block_hash_fails() {
    let reciever = startup::create_new_utxo_address();
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let private_key = jcli_wrapper::assert_key_generate_default();

    let witness = Witness::new(
        "FAKE_GENESIS_HASH",
        FAKE_INPUT_TRANSACTION_ID,
        "utxo",
        &private_key,
        &0,
    );
    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_make_witness_fails(&witness, "invalid hex encoding for hash value");
}

#[test]
pub fn test_make_witness_with_wrong_transaction_id_hash_fails() {
    let reciever = startup::create_new_utxo_address();
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let private_key = jcli_wrapper::assert_key_generate_default();

    let witness = Witness::new(
        FAKE_GENESIS_HASH,
        "FAKE_INPUT_TRANSACTION_ID",
        "utxo",
        &private_key,
        &0,
    );
    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_make_witness_fails(&witness, "invalid hex encoding for hash value");
}

#[test]
pub fn test_make_witness_for_utxo() {
    let reciever = startup::create_new_utxo_address();
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let private_key = jcli_wrapper::assert_key_generate_default();

    let witness = Witness::new(
        FAKE_GENESIS_HASH,
        FAKE_INPUT_TRANSACTION_ID,
        "utxo",
        &private_key,
        &0,
    );
    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_make_witness(&witness);
}

#[test]
pub fn test_make_witness_for_account() {
    let reciever = startup::create_new_utxo_address();
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let private_key = jcli_wrapper::assert_key_generate_default();

    let witness = Witness::new(
        FAKE_GENESIS_HASH,
        FAKE_ACCOUNT_ADDRESS,
        "account",
        &private_key,
        &0,
    );
    transaction_wrapper
        .assert_add_account(&FAKE_ACCOUNT_ADDRESS, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_make_witness_fails(&witness, "invalid hex encoding for hash value");
}

#[test]
pub fn test_make_witness_for_legacy_utxo() {
    let reciever = startup::create_new_utxo_address();
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let private_key = jcli_wrapper::assert_key_generate_default();

    let witness = Witness::new(
        FAKE_GENESIS_HASH,
        FAKE_INPUT_TRANSACTION_ID,
        "legacy-utxo",
        &private_key,
        &0,
    );
    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_make_witness_fails(&witness, "not yet implemented");
}

#[test]
pub fn test_cannot_seal_transaction_with_too_many_witnesses() {
    let reciever = startup::create_new_utxo_address();

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let witness_1 = transaction_wrapper.create_witness_default("utxo");
    let witness_2 = transaction_wrapper.create_witness_default("utxo");

    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_make_witness(&witness_1)
        .assert_make_witness(&witness_2)
        .assert_add_witness(&witness_1)
        .assert_add_witness_fail(&witness_2, "cannot add anymore witnesses");
}
