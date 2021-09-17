use std::path::PathBuf;

fn root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn tls_server_private_key() -> PathBuf {
    let mut tls_server_private_key = root_dir();
    tls_server_private_key.push("resources/tls/server.key");
    tls_server_private_key
}

pub fn tls_server_crt() -> PathBuf {
    let mut tls_server_crt = root_dir();
    tls_server_crt.push("resources/tls/server.crt");
    tls_server_crt
}

pub fn tls_ca_crt() -> PathBuf {
    let mut tls_ca_crt = root_dir();
    tls_ca_crt.push("resources/tls/ca.crt");
    tls_ca_crt
}
