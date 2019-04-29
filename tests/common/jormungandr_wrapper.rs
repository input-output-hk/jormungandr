#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;

use super::configuration;

/// Starts jormungandr node.
/// NOTE: it uses jormungandr which is already installed on current environment
/// (by cargo install command)
///
/// # Arguments
///
/// * `config_path` - Path to node config file
/// * `genesis_block_path` - Path to block file
/// # Example
///
/// use jormungandr_wrapper::start_jormungandr_node;
///
/// let config_path = "node.config";
/// let genesis_block_path = "block-0.bin";
/// let process = start_jormungandr_node()
///                       .spawn() {
///        Ok(process) => process,
///        Err(err)    => panic!("Running process error: {}", err),
/// };
///
pub fn start_jormungandr_node(config_path: &PathBuf, genesis_block_path: &PathBuf) -> Command {
    let mut command = Command::new(configuration::get_jormungandr_app().as_os_str());
    command
        .arg("--config")
        .arg(config_path.as_os_str())
        .arg("--genesis-block")
        .arg(genesis_block_path.as_os_str());
    command
}
