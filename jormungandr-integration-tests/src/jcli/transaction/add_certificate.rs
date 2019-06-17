use crate::common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use crate::common::startup;

const FAKE_INPUT_TRANSACTION_ID: &str =
    "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_CERTIFICATE: &str = "cert1qyzlw7xxpuw5ru9tmcejlptlu4jsfswwn2tk9k07ae6audxag4cmv4qak5p5nc4urfd35uunnwwcdlz9qec30nynpsm2lwm0kz5n982pqyqyq6y5ltkusja2wt5nswms7mycvvph5w26g8fuycycxqynpug87cu70gx6cuq43xca5cc034wz5vh43dzwml9v0tfrtlzr4qdpwhm4zcrqfhytdc";

#[test]
pub fn cannot_add_certificate_after_transaction_is_finalized() {
    let sender = startup::create_new_utxo_address();
    let reciever = startup::create_new_utxo_address();

    let mut builder = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);

    builder
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &200)
        .assert_add_output(&reciever.address, &150)
        .assert_finalize_with_spending_address(&sender.address)
        .assert_add_certificate_fails(
            &FAKE_CERTIFICATE,
            "adding certificate to finalizing transaction is not valid",
        );
}

#[test]
pub fn add_certificate_changes_transaction_id_after_seal() {
    let sender = startup::create_new_utxo_address();
    let reciever = startup::create_new_utxo_address();

    let mut builder = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);

    builder
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &200)
        .assert_add_output(&reciever.address, &150)
        .assert_finalize_with_spending_address(&sender.address)
        .seal_with_witness_deafult(&sender.private_key, "utxo")
        .assert_add_certificate_fails(
            &FAKE_CERTIFICATE,
            "adding certificate to sealed transaction is not valid",
        );
}
