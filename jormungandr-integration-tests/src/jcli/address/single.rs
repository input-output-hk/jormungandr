use crate::common::jcli_wrapper;
use crate::common::jcli_wrapper::Discrimination;
use crate::common::process_assert;

#[test]
pub fn test_utxo_address_made_of_ed25519_extended_key() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let utxo_address = jcli_wrapper::assert_address_single(&public_key, Discrimination::Test);
    assert_ne!(utxo_address, "", "generated utxo address is empty");
}

#[test]
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

    let delegation_address =
        jcli_wrapper::assert_address_delegation(&public_key, &delegation_key, Discrimination::Test);
    assert_ne!(
        delegation_address, "",
        "generated delegation adress is empty"
    );
}

#[test]
pub fn test_delegation_address_is_the_same_as_public() {
    let correct_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada3e135";

    let private_key = jcli_wrapper::assert_key_with_seed_generate("ed25519Extended", &correct_seed);
    println!("private key: {}", &private_key);

    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let delegation_address =
        jcli_wrapper::assert_address_delegation(&public_key, &public_key, Discrimination::Test);
    assert_ne!(
        delegation_address, "",
        "generated delegation address is empty"
    );
}

#[test]
pub fn test_delegation_address_for_prod_discrimination() {
    let correct_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada3e135";

    let private_key = jcli_wrapper::assert_key_with_seed_generate("ed25519Extended", &correct_seed);
    println!("private key: {}", &private_key);

    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let delegation_address = jcli_wrapper::assert_address_delegation(
        &public_key,
        &public_key,
        Discrimination::Production,
    );
    assert_ne!(
        delegation_address, "",
        "generated delegation address is empty"
    );
}

#[test]
pub fn test_single_address_for_prod_discrimination() {
    let correct_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada3e135";

    let private_key = jcli_wrapper::assert_key_with_seed_generate("ed25519Extended", &correct_seed);
    println!("private key: {}", &private_key);

    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let delegation_address = jcli_wrapper::assert_address_delegation(
        &public_key,
        &public_key,
        Discrimination::Production,
    );
    assert_ne!(delegation_address, "", "generated single address is empty");
}

#[test]
pub fn test_account_address_for_prod_discrimination() {
    let correct_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada3e135";

    let private_key = jcli_wrapper::assert_key_with_seed_generate("ed25519Extended", &correct_seed);
    println!("private key: {}", &private_key);

    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let delegation_address = jcli_wrapper::assert_address_delegation(
        &public_key,
        &public_key,
        Discrimination::Production,
    );
    assert_ne!(delegation_address, "", "generated account address is empty");
}
#[test]
pub fn test_utxo_address_made_of_incorrect_ed25519_extended_key() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let mut public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    public_key.push('A');

    // Assertion changed due to issue #306. After fix please change it to correct one
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_address_single_command(&public_key, Discrimination::Test),
        "Failed to parse bech32, invalid data format",
    );
}

#[test]
pub fn test_delegation_address_made_of_random_string() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    println!("public key: {}", &public_key);

    let delegation_key = "adfasdfasdfdasfasdfadfasdf";

    // Assertion changed due to issue #306. After fix please change it to correct one
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_address_delegation_command(
            &public_key,
            &delegation_key,
            Discrimination::Test,
        ),
        "Failed to parse bech32, invalid data format",
    );
}

#[test]
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
        jcli_wrapper::jcli_commands::get_address_delegation_command(
            &public_key,
            &delegation_key,
            Discrimination::Test,
        ),
        "Failed to parse bech32, invalid data format",
    );
}
