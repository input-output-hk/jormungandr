pub use crate::jormungandr::{
    Block0ConfigurationBuilder, JormungandrParams, NodeConfigBuilder, SecretModelFactory,
    TestConfig,
};
use std::{env, path::PathBuf};

/// Get jormungandr executable from current environment
pub fn get_jormungandr_app() -> PathBuf {
    const JORMUNGANDR_NAME: &str = env!("JORMUNGANDR_NAME");
    get_app_from_current_dir(JORMUNGANDR_NAME)
}

/// Get jcli executable from current environment
pub fn get_jcli_app() -> PathBuf {
    const JOR_CLI_NAME: &str = env!("JOR_CLI_NAME");
    get_app_from_current_dir(JOR_CLI_NAME)
}

/// Get explorer executable from current environment
pub fn get_explorer_app() -> PathBuf {
    const JOR_EXPLORER_NAME: &str = env!("JOR_EXPLORER_NAME");
    get_app_from_current_dir(JOR_EXPLORER_NAME)
}

/// Get executable from current environment
pub fn get_app_from_current_dir(app_name: &str) -> PathBuf {
    let mut path = get_working_directory();
    path.push(app_name);
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
    let mut output_directory: PathBuf = std::env::current_exe().unwrap();

    output_directory.pop();

    if output_directory.ends_with("deps") {
        output_directory.pop();
    }
    output_directory
}

pub fn get_openapi_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("doc");
    path.push("api");
    path.push("v0.yaml");
    path
}
