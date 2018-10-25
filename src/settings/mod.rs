mod command_arguments;
pub mod network;

use std::path::PathBuf;

pub use self::command_arguments::CommandArguments;

pub struct Settings {
    pub cmd_args: CommandArguments,

    pub network: network::Configuration,

    pub genesis_data_config: PathBuf,
}


impl Settings {
    pub fn load() -> Self {
        let command_arguments = CommandArguments::load();

        let network = network::Configuration {
            peer_nodes: command_arguments.connect_to.clone(),
            listen_to:  command_arguments.listen_addr.clone(),
        };

        Settings {
            genesis_data_config: command_arguments.genesis_data_config.clone(),
            network: network,
            cmd_args: command_arguments,
        }
    }

    pub fn get_log_level(&self) -> log::LevelFilter {
        let log_level = match self.cmd_args.verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };
        log_level
    }
}
