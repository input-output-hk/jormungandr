use super::file_utils;
use std::env;
use std::path::PathBuf;

pub mod genesis_model;
pub mod node_config_model;

/// Get jcli executable from current environment
pub fn get_jormungandr_app() -> PathBuf {
    let mut path = get_working_directory();
    path.push("JORMUNGANDR");
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
    path.push("JCLI");
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
fn get_working_directory() -> PathBuf {
    let mut output_directory: PathBuf = std::env::current_exe().unwrap().into();

    /// current exe directory is ./target/{profile}/deps/{app_name}.exe
    /// We would like to navigate to ./target/{profile}
    output_directory.pop();
    output_directory.pop();
    output_directory
}
