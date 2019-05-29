#![cfg(feature = "integration-test")]
use common::file_utils;
use common::jcli_wrapper;

#[test]
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

#[test]
pub fn test_to_bytes_for_non_existent_input_file() {
    let byte_key_file = file_utils::get_path_in_temp("byte_file");
    jcli_wrapper::assert_key_from_bytes_fails(&byte_key_file, "ed25519Extended", "file");
}

#[test]
pub fn test_to_bytes_for_invalid_key() {
    let byte_key_file = file_utils::create_file_in_temp("byte_file",
         "ed25519e_sk1kp80gevhccz8cnst6x97rmlc9n5fls2nmcqcjfn65vdktt0wy9f3zcf76hp7detq9sz8cmhlcyzw5h3ralf98rdwl4wcwcgaaqna3pgz9qgk0");
    let output_file = file_utils::get_path_in_temp("output_byte_file");
    jcli_wrapper::assert_key_to_bytes_fails(&byte_key_file, &output_file, "invalid Bech32");
}
