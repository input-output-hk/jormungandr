use assert_fs::TempDir;
use jormungandr_lib::interfaces::Tls;
use jormungandr_testing_utils::testing::{
    jormungandr::{ConfigurationBuilder, Starter, StartupVerificationMode},
    resources,
};

#[test]
#[cfg(any(unix, windows))]
pub fn test_rest_tls_config() {
    let temp_dir = TempDir::new().unwrap().into_persistent();
    let prv_key_file = resources::tls_server_private_key();
    let server_crt_file = resources::tls_server_crt();
    let ca_crt_file = resources::tls_ca_crt();

    let config = ConfigurationBuilder::new()
        .with_rest_tls_config(Tls {
            cert_file: server_crt_file.as_os_str().to_str().unwrap().to_owned(),
            priv_key_file: prv_key_file.as_os_str().to_str().unwrap().to_owned(),
        })
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .verify_by(StartupVerificationMode::Log)
        .start()
        .unwrap();
    println!("Bootstrapped");
    jormungandr.assert_no_errors_in_log();

    println!(
        "{:?}",
        jormungandr.secure_rest(&ca_crt_file).stats().unwrap()
    );
}
