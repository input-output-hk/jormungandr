#![allow(dead_code)]

extern crate lazy_static;
extern crate rand;

use self::lazy_static::lazy_static;
use self::rand::Rng;
use super::file_utils;
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};

pub mod genesis_model;
pub mod jormungandr_config;
pub mod node_config_model;
pub mod secret_model;

/// Get jormungandr executable from current environment
pub fn get_jormungandr_app() -> PathBuf {
    const JORMUNGANDR_NAME: &'static str = env!("JORMUNGANDR_NAME");
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
    const JOR_CLI_NAME: &'static str = env!("JOR_CLI_NAME");
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
    let mut output_directory: PathBuf = std::env::current_exe().unwrap().into();

    output_directory.pop();
    output_directory.pop();
    output_directory
}

pub fn get_openapi_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push("doc");
    path.push("openapi.yaml");
    path
}

lazy_static! {
    static ref NEXT_AVAILABLE_PORT_NUMBER: AtomicU16 = {
        let initial_port = rand::thread_rng().gen_range(6000, 10999);
        AtomicU16::new(initial_port)
    };
}

pub fn get_available_port() -> u16 {
    NEXT_AVAILABLE_PORT_NUMBER.fetch_add(1, Ordering::SeqCst)
}
