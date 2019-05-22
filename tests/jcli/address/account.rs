#![cfg(feature = "integration-test")]

use common::jcli_wrapper;
use common::process_assert;

#[test]
pub fn test_account_address_made_of_incorrect_ed25519_extended_key() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let mut public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    public_key.remove(20);

    // Assertion changed due to issue #306. After fix please change it to correct one
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_address_account_command_default(&public_key),
        "Failed to parse bech32, invalid data format",
    );
}

#[test]
pub fn test_account_address_made_of_ed25519_extended_key() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let account_address = jcli_wrapper::assert_address_account_default(&public_key);
    assert_ne!(account_address, "", "generated account address is empty");
}
