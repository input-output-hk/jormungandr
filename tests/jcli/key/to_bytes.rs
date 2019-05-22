#![cfg(feature = "integration-test")]
extern crate bytes;

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
