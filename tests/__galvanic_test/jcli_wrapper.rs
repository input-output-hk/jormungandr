use std::process::Command;
use std::path::PathBuf;

use crate::__galvanic_test::configuration;

fn get_jcli_app_path() -> PathBuf {	
    let jcli : PathBuf = configuration::get_jcli_app_variable_as_path_from_os();
    jcli	
}

/// Run genesis encode command. NOTE: it uses jcli which is already installed on current environment
/// (by cargo install command)
///
/// # Arguments
///
/// * `genesis_yaml_fle_path` - Path to genesis yaml file
/// * `path_to_output_block` - Path to output block file 
///
pub fn run_genesis_encode_command(genesis_yaml_fle_path: &PathBuf, path_to_output_block: &PathBuf ) -> Command {
    let mut command =  Command::new(get_jcli_app_path().as_os_str());
    command.arg("genesis")
        .arg("encode")
        .arg("--input")
        .arg(genesis_yaml_fle_path.as_os_str())
        .arg("--output")
        .arg(path_to_output_block.as_os_str());
    command   
}

/// Run genesis encode command. Uses default genesis yaml path 
/// NOTE: it uses jcli which is already installed on current environment
/// (by cargo install command)
///
/// # Arguments
///
/// * `path_to_output_block` - Path to output block file 
///
pub fn run_genesis_encode_command_default(path_to_output_block: &PathBuf ) -> Command {
    let mut command =  Command::new(get_jcli_app_path().as_os_str());
    command.arg("genesis")
        .arg("encode")
        .arg("--input")
        .arg(configuration::get_genesis_yaml_path().as_os_str())
        .arg("--output")
        .arg(path_to_output_block.as_os_str());
    command   
}

/// Run rest  stat command. Uses [default host and port](super::test_const::JORMUNGANDR_ADDRESS)
/// NOTE: it uses jcli which is already installed on current environment
/// (by cargo install command)
///
///
pub fn run_rest_stats_command_default() -> Command {
    let mut command =  Command::new(get_jcli_app_path().as_os_str());
    command.arg("rest")
        .arg("v0")
        .arg("node")
        .arg("stats")
        .arg("get")
        .arg("-h")
        .arg(&configuration::JORMUNGANDR_ADDRESS);
    command   
}