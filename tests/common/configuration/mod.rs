use super::file_utils;
use std::env;
use std::path::PathBuf;

pub mod genesis_model;
pub mod node_config_model;

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
