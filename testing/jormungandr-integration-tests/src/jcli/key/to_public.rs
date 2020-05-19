use crate::common::jcli_wrapper;

#[test]
pub fn test_key_to_public() {
    let private_key = "ed25519_sk1357nu8uaxvdekg6uhqmdd0zcd3tjv3qq0p2029uk6pvfxuks5rzstp5ceq";
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    assert_ne!(public_key, "", "generated key is empty");
}

#[test]
pub fn test_key_to_public_invalid_key() {
    jcli_wrapper::assert_key_to_public_fails(
        "ed2551ssss9_sk1357nu8uaxvdekg6uhqmdd0zcd3tjv3qq0p2029uk6pvfxuks5rzstp5ceq",
        "invalid checksum",
    );
}

#[test]
pub fn test_key_to_public_invalid_chars_key() {
    jcli_wrapper::assert_key_to_public_fails(
        "node:: ed2551ssss9_sk1357nu8uaxvdekg6uhqmdd0zcd3tjv3qq0p2029uk6pvfxuks5rzstp5ceq",
        "invalid character",
    );
}

#[test]
pub fn test_private_key_to_public_key() {
    let private_key = jcli_wrapper::assert_key_generate("Ed25519Extended");
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    assert_ne!(public_key, "", "generated key is empty");
}
