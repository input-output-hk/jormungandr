mod common;

extern crate bytes;
use bytes::Bytes;
use common::configuration::genesis_model::GenesisYaml;
use common::file_utils;
use common::jcli_wrapper;
use common::process_assert;
use common::process_utils;
use common::process_utils::output_extensions::ProcessOutput;
use common::startup;

#[test]
#[cfg(feature = "integration-test")]
pub fn test_utxo_address_made_of_ed25519_extended_key() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let utxoAddress = jcli_wrapper::assert_address_single_default(&public_key);
    assert_ne!(utxoAddress, "", "generated utxo address is empty");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_account_address_made_of_ed25519_extended_key() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let accountAddress = jcli_wrapper::assert_address_account_default(&public_key);
    assert_ne!(accountAddress, "", "generated account address is empty");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_delegation_address_made_of_ed25519_extended_seed_key() {
    let correct_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada3e135";

    let private_key = jcli_wrapper::assert_key_with_seed_generate("ed25519Extended", &correct_seed);
    println!("private key: {}", &private_key);

    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let private_key = jcli_wrapper::assert_key_with_seed_generate("ed25519Extended", &correct_seed);
    println!("private delegation key: {}", &private_key);
    let delegation_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("delegation key: {}", &delegation_key);

    let delegationAddress =
        jcli_wrapper::assert_address_delegation_default(&public_key, &delegation_key);
    assert_ne!(
        delegationAddress, "",
        "generated delegation adress is empty"
    );
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_delegation_address_is_the_same_as_public() {
    let correct_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada3e135";

    let private_key = jcli_wrapper::assert_key_with_seed_generate("ed25519Extended", &correct_seed);
    println!("private key: {}", &private_key);

    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let delegationAddress =
        jcli_wrapper::assert_address_delegation_default(&public_key, &public_key);
    assert_ne!(
        delegationAddress, "",
        "generated delegation adress is empty"
    );
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_utxo_address_made_of_incorrect_ed25519_extended_key() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let mut public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    public_key.push('A');

    // Assertion changed due to issue #306. After fix please change it to correct one
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_address_single_command_default(&public_key),
        "thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: MixedCase",
    );
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_account_address_made_of_incorrect_ed25519_extended_key() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let mut public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    public_key.remove(20);

    // Assertion changed due to issue #306. After fix please change it to correct one
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_address_account_command_default(&public_key),
        "thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: InvalidChecksum",
    );
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_delegation_address_made_of_random_string() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let mut public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let delegation_key = "adfasdfasdfdasfasdfadfasdf";

    // Assertion changed due to issue #306. After fix please change it to correct one
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_address_delegation_command_default(
            &public_key,
            &delegation_key,
        ),
        "thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: MissingSeparator",
    );
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_delegation_address_made_of_incorrect_public_ed25519_extended_key() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let mut public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private delegation key: {}", &private_key);
    let delegation_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("delegation key: {}", &delegation_key);

    public_key.push('A');

    // Assertion changed due to issue #306. After fix please change it to correct one
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_address_delegation_command_default(
            &public_key,
            &delegation_key,
        ),
        "thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: MixedCase",
    );
}
