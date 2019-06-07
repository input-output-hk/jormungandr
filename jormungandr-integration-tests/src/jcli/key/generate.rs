use crate::common::jcli_wrapper;
use crate::common::process_assert;

#[test]
pub fn test_ed25519_key_generation() {
    let generated_key = jcli_wrapper::assert_key_generate("ed25519");
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
pub fn test_ed25519_uppercase_key_generation() {
    let generated_key = jcli_wrapper::assert_key_generate("ED25519EXTENDED");
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
pub fn test_ed25510bip32_key_generation() {
    let generated_key = jcli_wrapper::assert_key_generate("Ed25519Bip32");
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
pub fn test_ed25519extended_key_generation() {
    let generated_key = jcli_wrapper::assert_key_generate("Ed25519Extended");
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
pub fn test_curve25519_2hashdh_key_generation() {
    let generated_key = jcli_wrapper::assert_key_generate("Curve25519_2HashDH");
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
pub fn test_sumed25519_12_key_generation() {
    let generated_key = jcli_wrapper::assert_key_generate("SumEd25519_12");
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
pub fn test_unknown_key_type_generation() {
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_key_generate_command("unknown"),
        "Invalid value for '--type <key_type>'",
    );
}

#[test]
pub fn test_key_with_seed_generation() {
    let correct_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada3e135";
    let generated_key =
        jcli_wrapper::assert_key_with_seed_generate("Ed25519Extended", &correct_seed);
    assert_ne!(generated_key, "", "generated key is empty");
}

#[test]
pub fn test_key_with_too_short_seed_generation() {
    let too_short_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada";
    test_key_invalid_seed_length(&too_short_seed);
}

#[test]
pub fn test_key_with_too_long_seed_generation() {
    let too_long_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcada0234212";
    test_key_invalid_seed_length(&too_long_seed);
}

fn test_key_invalid_seed_length(seed: &str) -> () {
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_key_generate_with_seed_command("Ed25519Extended", &seed),
        "invalid seed length, expected 32 bytes but received",
    );
}

#[test]
pub fn test_key_with_seed_with_unknown_symbol_generation() {
    let incorrect_seed = "73855612722627931e20c850f8ad53eb04c615c7601a95747be073dcay";
    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_key_generate_with_seed_command(
            "Ed25519Extended",
            &incorrect_seed,
        ),
        "error: Invalid value for '--seed <SEED>': Unknown symbol at byte index",
    );
}
