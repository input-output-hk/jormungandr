use jormungandr_testing_utils::testing::jcli::JCli;

#[test]
pub fn test_key_to_public() {
    let jcli: JCli = Default::default();
    let private_key = "ed25519_sk1357nu8uaxvdekg6uhqmdd0zcd3tjv3qq0p2029uk6pvfxuks5rzstp5ceq";
    let public_key = jcli.key().convert_to_public_string(private_key.to_owned());
    assert_ne!(public_key, "", "generated key is empty");
}

#[test]
pub fn test_key_to_public_invalid_key() {
    let jcli: JCli = Default::default();
    jcli.key().convert_to_public_string_expect_fail(
        "ed2551ssss9_sk1357nu8uaxvdekg6uhqmdd0zcd3tjv3qq0p2029uk6pvfxuks5rzstp5ceq",
        "invalid checksum",
    );
}

#[test]
pub fn test_key_to_public_invalid_chars_key() {
    let jcli: JCli = Default::default();
    jcli.key().convert_to_public_string_expect_fail(
        "node:: ed2551ssss9_sk1357nu8uaxvdekg6uhqmdd0zcd3tjv3qq0p2029uk6pvfxuks5rzstp5ceq",
        "invalid character",
    );
}

#[test]
pub fn test_private_key_to_public_key() {
    let jcli: JCli = Default::default();
    let private_key = jcli.key().generate("Ed25519Extended");
    let public_key = jcli.key().convert_to_public_string(&private_key);
    assert_ne!(public_key, "", "generated key is empty");
}
