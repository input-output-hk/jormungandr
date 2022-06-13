use assert_fs::{prelude::*, NamedTempFile};
use jormungandr_automation::jcli::JCli;

#[test]
pub fn test_key_from_bytes_ed25519() {
    transform_key_to_bytes_and_back("ed25519");
}

#[test]
pub fn test_key_from_bytes_curve25519_2hashdh() {
    transform_key_to_bytes_and_back("RistrettoGroup2HashDh");
}

#[test]
pub fn test_key_from_bytes_sumed25519_12() {
    transform_key_to_bytes_and_back("sumed25519_12");
}

#[test]
pub fn test_key_from_bytes_ed25510bip32() {
    transform_key_to_bytes_and_back("Ed25519Bip32");
}

fn transform_key_to_bytes_and_back(key_type: &str) {
    let jcli: JCli = Default::default();

    let private_key = jcli.key().generate(key_type);
    let byte_key_file = NamedTempFile::new("byte_file").unwrap();
    jcli.key()
        .dump_bytes_to_file(&private_key, byte_key_file.path());
    let key_after_transformation = jcli
        .key()
        .convert_from_bytes_string(key_type, byte_key_file.path());

    assert_eq!(
        &private_key, &key_after_transformation,
        "orginal and key after transformation are differnt '{}' vs '{}'",
        &private_key, &key_after_transformation
    );
}

#[test]
pub fn test_from_bytes_for_invalid_key() {
    let jcli: JCli = Default::default();
    let byte_key_file = NamedTempFile::new("byte_file").unwrap();
    byte_key_file.write_str(
        "ed25519e_sk1kp80gevhccz8cnst6x97rmlc9n5fls2nmcqcjfn65vdktt0wy9f3zcf76hp7detq9sz8cmhlcyzw5h3ralf98rdwl4wcwcgaaqna3pgz9qgk0").unwrap();
    jcli.key().convert_from_bytes_string_expect_fail(
        "ed25519Extended",
        byte_key_file.path(),
        "Odd number of digits",
    );
}

#[test]
pub fn test_from_bytes_for_unknown_key() {
    let jcli: JCli = Default::default();
    let byte_key_file = NamedTempFile::new("byte_file").unwrap();
    byte_key_file.write_str(
        "ed25519e_sk1kp80gevhccz8cnst6x97rmlc9n5fls2nmcqcjfn65vdktt0wy9f3zcf76hp7detq9sz8cmhlcyzw5h3ralf98rdwl4wcwcgaaqna3pgz9qgk0").unwrap();
    jcli.key().convert_from_bytes_string_expect_fail(
        "ed25519Exten",
        byte_key_file.path(),
        "Invalid value for '--type <key-type>':",
    );
}
