mod config;
pub mod network;

use self::config::ConfigLogSettings;
pub use self::config::{Genesis, GenesisConstants, Rest};
use self::network::Protocol;
use crate::rest::Error as RestError;
use crate::settings::command_arguments::*;
use crate::settings::logging::LogSettings;

use std::{collections::BTreeMap, fs::File, path::PathBuf};

custom_error! {pub Error
   ConfigIo { source: std::io::Error } = "Cannot read the node configuration file: {source}",
   Config { source: serde_yaml::Error } = "Error while parsing the node configuration file: {source}",
   Rest { source: RestError } = "The Rest configuration is invalid: {source}",
}

/// Overall Settings for node
pub struct Settings {
    pub network: network::Configuration,

    pub storage: Option<PathBuf>,

    pub block_0: Block0Info,

    pub leadership: Option<PathBuf>,

    pub log_settings: LogSettings,

    pub rest: Option<Rest>,
}

impl Settings {
    /// Load the settings
    /// - from the command arguments
    /// - from the config
    ///
    /// This function will print&exit if anything is not as it should be.
    pub fn load(command_line: &CommandLine) -> Result<Self, Error> {
        let command_arguments = &command_line.start_arguments;
        let config: config::Config = {
            let mut file = File::open(command_arguments.node_config.clone())?;
            serde_yaml::from_reader(&mut file)?
        };

        let network = generate_network(&command_arguments, &config);
        let log_settings = generate_log_settings(&command_line, &config);

        let storage = match (command_arguments.storage.as_ref(), config.storage) {
            (Some(path), _) => Some(path.clone()),
            (None, Some(path)) => Some(path.clone()),
            (None, None) => None,
        };

        let secret = if command_arguments.without_leadership {
            None
        } else {
            match (command_arguments.secret.as_ref(), config.secret_file) {
                (Some(path), _) => Some(path.clone()),
                (None, Some(path)) => Some(path.clone()),
                (None, None) => {
                    warn!("Node started without path to the stored secret keys, just like starting with `--without-leadership'");
                    None
                }
            }
        };

        Ok(Settings {
            storage: storage,
            block_0: command_arguments.block_0.clone(),
            network: network,
            leadership: secret,
            log_settings: log_settings,
            rest: config.rest,
        })
    }
}

fn generate_log_settings(command_arguments: &CommandLine, config: &config::Config) -> LogSettings {
    let level = if command_arguments.verbose == 0 {
        match config.logger {
            Some(ConfigLogSettings {
                verbosity: Some(v),
                format: _,
            }) => v.clone(),
            _ => 0,
        }
    } else {
        command_arguments.verbose
    };
    LogSettings {
        verbosity: match level {
            0 => slog::Level::Info,
            1 => slog::Level::Debug,
            _ => slog::Level::Trace,
        },
        format: command_arguments.log_format.clone(),
    }
}

fn generate_network(
    _command_arguments: &StartArguments,
    config: &config::Config,
) -> network::Configuration {
    let public_address = config.peer_2_peer.public_access.clone();
    network::Configuration {
        public_address: public_address,
        trusted_addresses: config.peer_2_peer.trusted_peers.clone().unwrap_or(vec![]),
        protocol: Protocol::Grpc,
        subscriptions: config
            .peer_2_peer
            .topics_of_interests
            .clone()
            .unwrap_or(BTreeMap::new()),
        timeout: std::time::Duration::from_secs(15),
    }
}
