use crate::common::{
    file_assert, file_utils,
    jormungandr::{ConfigurationBuilder, Starter},
};
use jormungandr_lib::{interfaces::Tls, testing::Openssl};
#[test]
#[cfg(any(unix, windows))]
pub fn test_rest_tls_config() {
    let openssl = Openssl::new().expect("no openssla installed.");
    let prv_key_file = file_utils::get_path_in_temp("prv.key");
    let csr_cert_file = file_utils::get_path_in_temp("cert.csr");
    let cert_file = file_utils::get_path_in_temp("cert.crt");

    println!(
        "{}",
        openssl
            .genrsa(2048, &prv_key_file)
            .expect("cannot generate private key.")
    );
    println!(
        "{}",
        openssl
            .pkcs8(&prv_key_file, &csr_cert_file)
            .expect("cannot wrap private key in PKC8")
    );
    println!(
        "{}",
        openssl
            .req(&prv_key_file, &csr_cert_file)
            .expect("cannot register a self-signed certificate for private key")
    );
    println!(
        "{}",
        openssl
            .x509(&prv_key_file, &csr_cert_file, &cert_file)
            .expect("cannot generate a self-signed certificate for private key")
    );

    file_assert::assert_file_exists_and_not_empty(&prv_key_file);
    file_assert::assert_file_exists_and_not_empty(&cert_file);

    let config = ConfigurationBuilder::new()
        .with_rest_tls_config(Tls {
            cert_file: cert_file.as_os_str().to_str().unwrap().to_owned(),
            priv_key_file: prv_key_file.as_os_str().to_str().unwrap().to_owned(),
        })
        .build();

    let jormungandr = Starter::new().config(config).start().unwrap();

    jormungandr.assert_no_errors_in_log();

    println!("{:?}", jormungandr.secure_rest(cert_file).stats().unwrap());
}
