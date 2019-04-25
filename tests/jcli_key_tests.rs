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
pub fn test_ed25519_key_generation() {
    let generated_key = jcli_wrapper::assert_key_generate("ed25519");
    assert_ne!(generated_key, "", "generated key is empty");
}
#[test]
#[cfg(feature = "integration-test")]
pub fn test_ed25510bip32_key_generation() {
    let generated_key = jcli_wrapper::assert_key_generate("Ed25519Bip32");
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_ed25519extended_key_generation() {
    let generated_key = jcli_wrapper::assert_key_generate("Ed25519Extended");
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_curve25519_2hashdh_key_generation() {
    let generated_key = jcli_wrapper::assert_key_generate("Curve25519_2HashDH");
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_fake_mm_key_generation() {
    let generated_key = jcli_wrapper::assert_key_generate("FakeMMM");
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_unknown_key_type_generation() {
    let output = process_utils::run_process_and_get_output(
        jcli_wrapper::jcli_commands::get_key_generate_command("unknown"),
    );
    let actual = output.err_as_single_line();
    let expected_part = "Invalid value for '--type <key_type>'";

    assert_eq!(
        actual.contains(&expected_part),
        true,
        "message : '{}' does not contain expected part '{}'",
        &actual,
        &expected_part
    );

    process_assert::assert_process_failed(output);
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_key_with_seed_generation() {
    let correct_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada3e135";
    let generated_key =
        jcli_wrapper::assert_key_with_seed_generate("Ed25519Extended", &correct_seed);
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_key_with_too_short_seed_generation() {
    let too_short_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada";
    test_key_invalid_seed_length(&too_short_seed);
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_key_with_too_long_seed_generation() {
    let too_long_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada0234212";
    test_key_invalid_seed_length(&too_long_seed);
}

fn test_key_invalid_seed_length(seed: &str) -> () {
    let output = process_utils::run_process_and_get_output(
        jcli_wrapper::jcli_commands::get_key_generate_with_seed_command("Ed25519Extended", &seed),
    );
    let actual = output.err_as_single_line();
    let expected_part = "Invalid seed length, expected 32 bytes but received";

    assert_eq!(
        actual.contains(&expected_part),
        true,
        "message : '{}' does not contain expected part '{}'",
        &actual,
        &expected_part
    );

    process_assert::assert_process_failed(output);
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_key_with_seed_with_unknown_symbol_generation() {
    let incorrect_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcay";
    let output = process_utils::run_process_and_get_output(
        jcli_wrapper::jcli_commands::get_key_generate_with_seed_command(
            "Ed25519Extended",
            &incorrect_seed,
        ),
    );
    let actual = output.err_as_single_line();
    let expected_part = "error: Invalid value for '--seed <SEED>': Unknown symbol at byte index";

    assert_eq!(
        actual.contains(&expected_part),
        true,
        "message : '{}' does not contain expected part '{}'",
        &actual,
        &expected_part
    );

    process_assert::assert_process_failed(output);
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_key_to_public() {
    let private_key = "ed25519_sk1357nu8uaxvdekg6uhqmdd0zcd3tjv3qq0p2029uk6pvfxuks5rzstp5ceq";
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    assert_ne!(public_key, "", "generated key is empty");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_key_to_public_invalid_key() {
    let private_key = "ed2551ssss9_sk1357nu8uaxvdekg6uhqmdd0zcd3tjv3qq0p2029uk6pvfxuks5rzstp5ceq";
    let output = process_utils::run_process_and_get_output(
        jcli_wrapper::jcli_commands::get_key_to_public_command(&private_key),
    );
    let actual = output.err_as_single_line();
    let expected_part = "invalid checksum";

    assert_eq!(
        actual.contains(&expected_part),
        true,
        "message : '{}' does not contain expected part '{}'",
        &actual,
        &expected_part
    );

    process_assert::assert_process_failed(output);
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_private_key_to_public_key() {
    let private_key = jcli_wrapper::assert_key_generate("Ed25519Extended");
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    assert_ne!(public_key, "", "generated key is empty");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_key_from_and_to_bytes() {
    let private_key = jcli_wrapper::assert_key_generate("Ed25519Extended");
    let byte_key_file = file_utils::create_empty_file_in_temp("byte_file");
    jcli_wrapper::assert_key_to_bytes(&private_key, &byte_key_file);
    let key_after_transformation =
        jcli_wrapper::assert_key_from_bytes(&byte_key_file, "Ed25519Extended");

    assert_eq!(
        &private_key, &key_after_transformation,
        "orginal and key after transformation are differnt '{}' vs '{}'",
        &private_key, &key_after_transformation
    );
}
