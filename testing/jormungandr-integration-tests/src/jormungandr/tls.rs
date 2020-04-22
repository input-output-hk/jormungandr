use jormungandr_lib::testing::Openssl;

#[test]
pub fn test_openssl_version() {
    println!("{}", Openssl::new().unwrap().version().unwrap());
}
