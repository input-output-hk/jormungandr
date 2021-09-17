use jormungandr_testing_utils::testing::common::jcli::JCli;
use chain_addr::Discrimination;

#[test]
pub fn test_account_address_made_of_incorrect_ed25519_extended_key() {
    let jcli: JCli = Default::default();

    let private_key = jcli.key().generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let mut public_key = jcli.key().convert_to_public_string(&private_key);
    println!("public key: {}", &public_key);

    public_key.remove(20);

    // Assertion changed due to issue #306. After fix please change it to correct one
    jcli.address().account_expect_fail(
        &public_key,
        None,
        Discrimination::Test,
        "Failed to parse bech32, invalid data format",
    );
}

#[test]
pub fn test_account_address_made_of_ed25519_extended_key() {
    let jcli: JCli = Default::default();

    let private_key = jcli.key().generate("ed25519Extended");
    println!("private key: {}", &private_key);

    let public_key = jcli.key().convert_to_public_string(&private_key);
    println!("public key: {}", &public_key);

    let account_address = jcli
        .address()
        .account(&public_key, None, Discrimination::Test);
    assert_ne!(account_address, "", "generated account address is empty");
}
