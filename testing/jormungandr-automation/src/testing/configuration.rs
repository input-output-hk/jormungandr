use std::env;
use std::path::PathBuf;

pub use crate::jormungandr::{
    Block0ConfigurationBuilder, JormungandrParams, NodeConfigBuilder, SecretModelFactory,
    TestConfig,
};

/// Get jormungandr executable from current environment
pub fn get_jormungandr_app() -> PathBuf {
    const JORMUNGANDR_NAME: &str = env!("JORMUNGANDR_NAME");
    let mut path = get_working_directory();
    path.push(JORMUNGANDR_NAME);
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
    const JOR_CLI_NAME: &str = env!("JOR_CLI_NAME");
    let mut path = get_working_directory();
    path.push(JOR_CLI_NAME);
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
