use crate::common::jormungandr::{ConfigurationBuilder, Starter, StartupVerificationMode};
use assert_fs::TempDir;
use assert_fs::{assert::PathAssert, fixture::PathChild};
use jormungandr_lib::interfaces::Tls;
use jormungandr_testing_utils::testing::Openssl;

#[test]
#[ignore]
#[cfg(any(unix, windows))]
pub fn test_rest_tls_config() {
    let temp_dir = TempDir::new().unwrap().into_persistent();

    let openssl = Openssl::new().expect("no openssla installed.");
    let prv_key_file = temp_dir.child("prv.key");
    let pk8_key_file = temp_dir.child("prv.pk8");
    let csr_cert_file = temp_dir.child("cert.csr");
    let cert_file = temp_dir.child("cert.crt");
    let der_file = temp_dir.child("cert.der");

    println!(
        "{}",
        openssl
            .genrsa(2048, &prv_key_file)
            .expect("cannot generate private key.")
    );
    println!(
        "{}",
        openssl
            .pkcs8(&prv_key_file, &pk8_key_file)
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
    println!(
        "{}",
        openssl
            .convert_to_der(&cert_file, &der_file)
            .expect("cannot convert cert file to der file")
    );

    prv_key_file.assert(crate::predicate::file_exists_and_not_empty());
    cert_file.assert(crate::predicate::file_exists_and_not_empty());

    let config = ConfigurationBuilder::new()
        .with_rest_tls_config(Tls {
            cert_file: cert_file.path().as_os_str().to_str().unwrap().to_owned(),
            priv_key_file: pk8_key_file.path().as_os_str().to_str().unwrap().to_owned(),
        })
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .config(config)
        .verify_by(StartupVerificationMode::Log)
        .start()
        .unwrap();
    println!("Bootstrapped");
    jormungandr.assert_no_errors_in_log();

    println!("{:?}", jormungandr.secure_rest(&der_file).stats().unwrap());
}
