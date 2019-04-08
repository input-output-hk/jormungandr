use std::process::Command;
use super::resources_const::GENESIS_YAML_FILE_PATH;

/// Run genesis encode command. NOTE: it uses jcli which is already installed on current environment
/// (by cargo install command)
///
/// # Arguments
///
/// * `genesis_yaml_fle_path` - Path to genesis yaml file
/// * `path_to_output_block` - Path to output block file 
///
pub fn run_genesis_encode_command(genesis_yaml_fle_path: &str, path_to_output_block: &str ) -> Command {
        let mut command =  Command::new("jcli");
        command.arg("genesis")
            .arg("encode")
            .arg("--input")
            .arg(&genesis_yaml_fle_path)
            .arg("--output")
            .arg(&path_to_output_block);
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
pub fn run_genesis_encode_command_default(path_to_output_block: &str ) -> Command {
        let mut command =  Command::new("jcli");
        command.arg("genesis")
            .arg("encode")
            .arg("--input")
            .arg(&GENESIS_YAML_FILE_PATH)
            .arg("--output")
            .arg(&path_to_output_block);
         command   
}