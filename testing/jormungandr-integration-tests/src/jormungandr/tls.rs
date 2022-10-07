use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use jormungandr_automation::{
    jormungandr::{NodeConfigBuilder, StartupVerificationMode},
    testing::resources,
};
use jormungandr_lib::interfaces::Tls;

#[test]
#[cfg(any(unix, windows))]
pub fn test_rest_tls_config() {
    let temp_dir = TempDir::new().unwrap().into_persistent();
    let prv_key_file = resources::tls_server_private_key();
    let server_crt_file = resources::tls_server_crt();
    let ca_crt_file = resources::tls_ca_crt();

    let config = NodeConfigBuilder::default().with_rest_tls_config(Tls {
        cert_file: server_crt_file.as_os_str().to_str().unwrap().to_owned(),
        priv_key_file: prv_key_file.as_os_str().to_str().unwrap().to_owned(),
    });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_node_config(config)
        .build()
        .starter(temp_dir)
        .unwrap()
        .verify_by(StartupVerificationMode::Log)
        .start()
        .unwrap();

    jormungandr.assert_no_errors_in_log();

    println!(
        "{:?}",
        jormungandr.secure_rest(&ca_crt_file).stats().unwrap()
    );
}
