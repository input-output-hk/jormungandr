use assert_fs::{prelude::*, NamedTempFile, TempDir};
use jormungandr_automation::jcli::JCli;

#[test]
pub fn test_key_from_and_to_bytes() {
    let jcli: JCli = Default::default();
    let private_key = jcli.key().generate("Ed25519Extended");
    let byte_key_file = NamedTempFile::new("byte_file").unwrap();
    jcli.key()
        .dump_bytes_to_file(&private_key, byte_key_file.path());
    let key_after_transformation = jcli
        .key()
        .convert_from_bytes_string("Ed25519Extended", byte_key_file.path());

    assert_eq!(
        &private_key, &key_after_transformation,
        "orginal and key after transformation are differnt '{}' vs '{}'",
        &private_key, &key_after_transformation
    );
}

#[test]
pub fn test_to_bytes_for_non_existent_input_file() {
    let jcli: JCli = Default::default();
    let byte_key_file = NamedTempFile::new("byte_file").unwrap();
    jcli.key().convert_from_bytes_string_expect_fail(
        "ed25519Extended",
        byte_key_file.path(),
        "file",
    );
}

#[test]
pub fn test_to_bytes_for_invalid_key() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();
    let byte_key_file = temp_dir.child("byte_file");
    byte_key_file.write_str("ed25519e_sk1kp80gevhccz8cnst6x97rmlc9n5fls2nmcqcjfn65vdktt0wy9f3zcf76hp7detq9sz8cmhlcyzw5h3ralf98rdwl4wcwcgaaqna3pgz9qgk0").unwrap();
    let output_file = temp_dir.child("output_byte_file");
    jcli.key().convert_to_bytes_file_expect_fail(
        byte_key_file.path(),
        output_file.path(),
        "invalid Bech32",
    );
}
