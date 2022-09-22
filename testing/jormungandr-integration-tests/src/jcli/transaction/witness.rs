use chain_addr::Discrimination;
use chain_impl_mockchain::{account::SpendingCounter, header::BlockDate, testing::TestGen};
use jormungandr_automation::jcli::{JCli, Witness, WitnessData, WitnessType};
use jormungandr_lib::crypto::hash::Hash;
use std::path::PathBuf;

lazy_static::lazy_static! {
    static ref FAKE_GENESIS_HASH: Hash = TestGen::hash().into();
    static ref FAKE_INPUT_TRANSACTION_ID: Hash = TestGen::hash().into();
}

#[test]
pub fn test_utxo_transation_with_more_than_one_witness_per_input_is_rejected() {
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);

    let mut transaction_wrapper = JCli::default().transaction_builder(TestGen::hash().into());
    transaction_wrapper
        .new_transaction()
        .add_input(&FAKE_INPUT_TRANSACTION_ID, 0, "100")
        .add_output(&receiver.address_bech32(Discrimination::Test), 100.into())
        .set_expiry_date(BlockDate::first().into())
        .finalize();

    let witness1 = transaction_wrapper.create_witness_default(WitnessType::UTxO, None);
    transaction_wrapper
        .make_witness(&witness1)
        .add_witness(&witness1);

    let witness2 = transaction_wrapper.create_witness_default(WitnessType::UTxO, None);
    transaction_wrapper
        .make_witness(&witness2)
        .add_witness_expect_fail(
            &witness2,
            "too many witnesses in transaction to add another: 1, maximum is 1",
        );
}

#[test]
pub fn test_utxo_transation_with_address_type_witness_is_rejected() {
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);

    let mut transaction_wrapper = JCli::default().transaction_builder(TestGen::hash().into());

    transaction_wrapper
        .new_transaction()
        .add_input(&FAKE_INPUT_TRANSACTION_ID, 0, "100")
        .add_output(&receiver.address_bech32(Discrimination::Test), 100.into())
        .set_expiry_date(BlockDate::first().into())
        .finalize();

    let witness = transaction_wrapper.create_witness_default(WitnessType::Account, None);
    // FIXME: this is not a sensible behavior. Ideally jcli should reject such
    // malformed transaction, but we will leave this test here so as not to forget
    // about checking this
    transaction_wrapper.seal_with_witness(&witness).to_message();
}

#[test]
pub fn test_account_transation_with_utxo_type_witness_is_rejected() {
    let receiver = thor::Wallet::default();
    let sender = thor::Wallet::default();

    let mut transaction_wrapper = JCli::default().transaction_builder(TestGen::hash().into());
    transaction_wrapper
        .new_transaction()
        .add_account(&sender.address_bech32(Discrimination::Test), &100.into())
        .add_output(&receiver.address_bech32(Discrimination::Test), 100.into())
        .set_expiry_date(BlockDate::first().into())
        .finalize();
    let witness = transaction_wrapper.create_witness_default(WitnessType::UTxO, None);
    // FIXME: this is not a sensible behavior. Ideally jcli should reject such
    // malformed transaction, but we will leave this test here so as not to forget
    // about checking this
    transaction_wrapper.seal_with_witness(&witness).to_message();
}

#[test]
pub fn test_make_witness_with_unknown_type_fails() {
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let mut transaction_wrapper = JCli::default().transaction_builder(TestGen::hash().into());
    transaction_wrapper
        .new_transaction()
        .add_input(&FAKE_INPUT_TRANSACTION_ID, 0, "100")
        .add_output(&receiver.address_bech32(Discrimination::Test), 100.into())
        .set_expiry_date(BlockDate::first().into())
        .finalize();
    let witness = transaction_wrapper.create_witness_default(WitnessType::Unknown, None);
    transaction_wrapper.make_witness_expect_fail(&witness, "Invalid witness type");
}

#[test]
pub fn test_make_witness_with_invalid_private_key_fails() {
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let jcli: JCli = Default::default();

    let mut transaction_wrapper = JCli::default().transaction_builder(TestGen::hash().into());

    let mut private_key = jcli.key().generate_default();
    private_key.push('3');

    transaction_wrapper
        .new_transaction()
        .add_input(&FAKE_INPUT_TRANSACTION_ID, 0, "100")
        .add_output(&receiver.address_bech32(Discrimination::Test), 100.into())
        .set_expiry_date(BlockDate::first().into())
        .finalize();

    let witness = transaction_wrapper.create_witness(WitnessData::new_utxo(&private_key));
    transaction_wrapper
        .make_witness_expect_fail(&witness, "Failed to parse bech32, invalid data format");
}

#[test]
pub fn test_make_witness_with_non_existing_private_key_file_fails() {
    let jcli: JCli = Default::default();
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let mut transaction_wrapper = jcli.transaction_builder(TestGen::hash().into());
    let private_key = jcli.key().generate_default();
    transaction_wrapper
        .new_transaction()
        .add_input(&FAKE_INPUT_TRANSACTION_ID, 0, "100")
        .add_output(&receiver.address_bech32(Discrimination::Test), 100.into())
        .set_expiry_date(BlockDate::first().into())
        .finalize();
    let mut witness = Witness::new(
        transaction_wrapper.staging_dir(),
        &FAKE_GENESIS_HASH,
        &FAKE_INPUT_TRANSACTION_ID,
        WitnessType::UTxO,
        &private_key,
        None,
    );
    witness.private_key_path = PathBuf::from("a");
    transaction_wrapper.make_witness_expect_fail(&witness, "os error 2");
}

#[test]
pub fn test_account_transaction_different_lane_is_accepted() {
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);

    let mut transaction_wrapper = JCli::default().transaction_builder(TestGen::hash().into());

    transaction_wrapper
        .new_transaction()
        .add_input(&FAKE_INPUT_TRANSACTION_ID, 0, "100")
        .add_output(&receiver.address_bech32(Discrimination::Test), 100.into())
        .set_expiry_date(BlockDate::first().into())
        .finalize();
    let witness = transaction_wrapper
        .create_witness_default(WitnessType::Account, Some(SpendingCounter::new(2, 0)));
    transaction_wrapper.seal_with_witness(&witness).to_message();
}
