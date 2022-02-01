use chain_addr::Discrimination;
use jormungandr_automation::jcli::JCli;

#[test]
pub fn test_info_unknown_address_public_key() {
    let jcli: JCli = Default::default();

    let account_address = "48mDfYyQn21iyEPzCfkATEHTwZBcZJqXhRJezmswfvc6Ne89u1axXsiazmgd7SwT8VbafbVnCvyXhBSMhSkPiCezMkqHC4dmxRahRC86SknFu6JF6hwSg8";
    jcli.address()
        .info_expect_fail(account_address, "invalid internal encoding");
}

#[test]
pub fn test_info_account_address() {
    let jcli: JCli = Default::default();

    let private_key = jcli.key().generate("ed25519Extended");
    let public_key = jcli.key().convert_to_public_string(&private_key);
    let account_address = jcli
        .address()
        .account(&public_key, None, Discrimination::Test);
    let info = jcli.address().info(&account_address);
    assert_eq!(
        info.get("discrimination").unwrap(),
        "testing",
        "wrong discrimination"
    );
    assert_eq!(info.get("account").unwrap(), &public_key, "wrong address");
}

#[test]
pub fn test_info_account_address_for_prod() {
    let jcli: JCli = Default::default();

    let private_key = jcli.key().generate("ed25519Extended");
    let public_key = jcli.key().convert_to_public_string(&private_key);
    let account_address = jcli
        .address()
        .account(&public_key, None, Discrimination::Production);
    let info = jcli.address().info(&account_address);
    assert_eq!(
        info.get("discrimination").unwrap(),
        "production",
        "wrong discrimination"
    );
    assert_eq!(info.get("account").unwrap(), &public_key, "wrong address");
}

#[test]
pub fn test_info_delegation_address() {
    let jcli: JCli = Default::default();

    let private_key = jcli.key().generate("ed25519Extended");
    let public_key = jcli.key().convert_to_public_string(&private_key);

    let private_key = jcli.key().generate("ed25519Extended");
    let delegation_key = jcli.key().convert_to_public_string(&private_key);
    let account_address =
        jcli.address()
            .delegation(&public_key, &delegation_key, Discrimination::Test);
    let info = jcli.address().info(&account_address);
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
