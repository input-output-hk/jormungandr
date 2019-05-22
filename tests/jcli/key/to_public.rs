#![cfg(feature = "integration-test")]

use common::jcli_wrapper;
use common::process_assert;

#[test]
pub fn test_key_to_public() {
    let private_key = "ed25519_sk1357nu8uaxvdekg6uhqmdd0zcd3tjv3qq0p2029uk6pvfxuks5rzstp5ceq";
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    assert_ne!(public_key, "", "generated key is empty");
}

#[test]
pub fn test_key_to_public_invalid_key() {
    let private_key = "ed2551ssss9_sk1357nu8uaxvdekg6uhqmdd0zcd3tjv3qq0p2029uk6pvfxuks5rzstp5ceq";
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_key_to_public_command(&private_key),
        "invalid checksum",
    );
}

#[test]
pub fn test_key_to_public_invalid_chars_key() {
    let private_key =
        "node:: ed2551ssss9_sk1357nu8uaxvdekg6uhqmdd0zcd3tjv3qq0p2029uk6pvfxuks5rzstp5ceq";
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_key_to_public_command(&private_key),
        "invalid character",
    );
}

#[test]
pub fn test_private_key_to_public_key() {
    let private_key = jcli_wrapper::assert_key_generate("Ed25519Extended");
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    assert_ne!(public_key, "", "generated key is empty");
}
