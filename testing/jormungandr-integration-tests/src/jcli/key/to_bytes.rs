use crate::common::jcli_wrapper;

use assert_fs::prelude::*;
use assert_fs::{NamedTempFile, TempDir};

#[test]
pub fn test_key_from_and_to_bytes() {
    let private_key = jcli_wrapper::assert_key_generate("Ed25519Extended");
    let byte_key_file = NamedTempFile::new("byte_file").unwrap();
    jcli_wrapper::assert_key_to_bytes(&private_key, byte_key_file.path());
    let key_after_transformation =
        jcli_wrapper::assert_key_from_bytes(byte_key_file.path(), "Ed25519Extended");

    assert_eq!(
        &private_key, &key_after_transformation,
        "orginal and key after transformation are differnt '{}' vs '{}'",
        &private_key, &key_after_transformation
    );
}

#[test]
pub fn test_to_bytes_for_non_existent_input_file() {
    let byte_key_file = NamedTempFile::new("byte_file").unwrap();
    jcli_wrapper::assert_key_from_bytes_fails(byte_key_file.path(), "ed25519Extended", "file");
}

#[test]
pub fn test_to_bytes_for_invalid_key() {
    let temp_dir = TempDir::new().unwrap();
    let byte_key_file = temp_dir.child("byte_file");
    byte_key_file.write_str("ed25519e_sk1kp80gevhccz8cnst6x97rmlc9n5fls2nmcqcjfn65vdktt0wy9f3zcf76hp7detq9sz8cmhlcyzw5h3ralf98rdwl4wcwcgaaqna3pgz9qgk0").unwrap();
    let output_file = temp_dir.child("output_byte_file");
    jcli_wrapper::assert_key_to_bytes_fails(
        byte_key_file.path(),
        output_file.path(),
        "invalid Bech32",
    );
}
