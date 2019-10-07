use crate::common::jcli_wrapper;
use chain_addr::Discrimination;

#[test]
pub fn test_info_unknown_address_public_key() {
    let account_address = "48mDfYyQn21iyEPzCfkATEHTwZBcZJqXhRJezmswfvc6Ne89u1axXsiazmgd7SwT8VbafbVnCvyXhBSMhSkPiCezMkqHC4dmxRahRC86SknFu6JF6hwSg8";
    jcli_wrapper::assert_get_address_info_fails(&account_address, "invalid internal encoding");
}

#[test]
pub fn test_info_account_address() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let account_address = jcli_wrapper::assert_address_account(&public_key, Discrimination::Test);
    let info = jcli_wrapper::assert_get_address_info(&account_address);
    assert_eq!(
        info.get("discrimination").unwrap(),
        "testing",
        "wrong discrimination"
    );
    assert_eq!(info.get("account").unwrap(), &public_key, "wrong address");
}

#[test]
pub fn test_info_account_address_for_prod() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let account_address =
        jcli_wrapper::assert_address_account(&public_key, Discrimination::Production);
    let info = jcli_wrapper::assert_get_address_info(&account_address);
    assert_eq!(
        info.get("discrimination").unwrap(),
        "production",
        "wrong discrimination"
    );
    assert_eq!(info.get("account").unwrap(), &public_key, "wrong address");
}

#[test]
pub fn test_info_delegation_address() {
    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);

    let private_key = jcli_wrapper::assert_key_generate("ed25519Extended");
    let delegation_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let account_address =
        jcli_wrapper::assert_address_delegation(&public_key, &delegation_key, Discrimination::Test);
    let info = jcli_wrapper::assert_get_address_info(&account_address);
    assert_eq!(
        info.get("discrimination").unwrap(),
        "testing",
        "wrong discrimination"
    );
    assert_eq!(
        info.get("public key").unwrap(),
        &public_key,
        "wrong public key"
    );
    assert_eq!(
        info.get("group key").unwrap(),
        &delegation_key,
        "wrong group key"
    );
}
