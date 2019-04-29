#![allow(dead_code)]

use super::file_utils;
use std::env;
use std::path::PathBuf;

pub mod genesis_model;
pub mod node_config_model;

/// Get jcli executable from current environment
pub fn get_jormungandr_app() -> PathBuf {
    let mut path = get_working_directory();
    path.push("jormungandr");
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
    let mut path = get_working_directory();
    path.push("jcli");
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

/// Gets working directory
/// Uses std::env::current_exe() for this purpose.
/// Current exe directory is ./target/{profile}/deps/{app_name}.exe
/// Function returns ./target/{profile}
fn get_working_directory() -> PathBuf {
    let mut output_directory: PathBuf = std::env::current_exe().unwrap().into();

    output_directory.pop();
    output_directory.pop();
    output_directory
}
