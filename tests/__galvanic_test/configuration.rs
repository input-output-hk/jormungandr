use std::path::PathBuf;
use std::{env, env::VarError};

pub fn get_genesis_yaml_path() -> PathBuf {
    let path = PathBuf::from("./tests/resources/genesis/genesis.yaml");
    path
}

pub fn get_node_config_path() -> PathBuf {
    let path = PathBuf::from("./tests/resources/jormungandr/node.config");
    path
}

/// Get jcli executable from current environment
pub fn get_jormungandr_app() -> PathBuf {
    let mut path: PathBuf = env!("JORMUNGANDR").into();
    if cfg!(windows) {
        path.set_extension("exe");
    }
    assert!(
        path.is_file(),
        "File does not exist: {:?}, pwd: {:?}",
        path,
        env::current_dir()
    );
    path
}

/// Get jcli executable from current environment
pub fn get_jcli_app() -> PathBuf {
    let mut path: PathBuf = env!("JCLI").into();
    if cfg!(windows) {
        path.set_extension("exe");
    }
    assert!(
        path.is_file(),
        "File does not exist: {:?}, pwd: {:?}",
        path,
        env::current_dir()
    );
    path
}

pub const JORMUNGANDR_ADDRESS: &str = "http://127.0.0.1:8443/api";
