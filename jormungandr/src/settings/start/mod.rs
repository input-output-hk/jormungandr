mod config;
pub mod network;

use self::config::{Config, Leadership, Mempool};
pub use self::config::{Cors, Rest};
use self::network::Protocol;
use crate::rest::Error as RestError;
use crate::settings::logging::{self, LogFormat, LogOutput, LogSettings};
use crate::settings::{command_arguments::*, Block0Info};
use slog::{FilterLevel, Logger};

use std::{collections::BTreeMap, fs::File, path::PathBuf};

custom_error! {pub Error
   ConfigIo { source: std::io::Error } = "Cannot read the node configuration file: {source}",
   Config { source: serde_yaml::Error } = "Error while parsing the node configuration file: {source}",
   Rest { source: RestError } = "The Rest configuration is invalid: {source}",
   MissingNodeConfig = "--config is mandatory to start the node",
   ExpectedBlock0Info = "Cannot start the node without the information to retrieve the genesis block",
   TooMuchBlock0Info = "Use only `--genesis-block-hash' or `--genesis-block'",
}

/// Overall Settings for node
pub struct Settings {
    pub network: network::Configuration,
    pub storage: Option<PathBuf>,
    pub block_0: Block0Info,
    pub secrets: Vec<PathBuf>,
    pub rest: Option<Rest>,
    pub mempool: Mempool,
    pub leadership: Leadership,
    pub explorer: bool,
}

pub struct RawSettings {
    command_line: CommandLine,
    config: Config,
}

impl RawSettings {
    pub fn load(command_line: CommandLine) -> Result<Self, Error> {
        let config_file = if let Some(node_config) = &command_line.start_arguments.node_config {
            File::open(node_config)?
        } else {
            return Err(Error::MissingNodeConfig);
        };
        let config = serde_yaml::from_reader(config_file)?;
        Ok(Self {
            command_line,
            config,
        })
    }

    pub fn to_logger(&self) -> Result<Logger, logging::Error> {
        LogSettings {
            level: self.logger_level(),
            format: self.logger_format(),
            output: self.logger_output(),
        }
        .to_logger()
    }

    fn logger_level(&self) -> FilterLevel {
        let cmd_level = self.command_line.log_level.clone();
        let config_log = self.config.log.as_ref();
        let config_level = config_log.and_then(|log| log.level.clone());
        cmd_level.or(config_level).unwrap_or(FilterLevel::Info)
    }

    fn logger_format(&self) -> LogFormat {
        let cmd_format = self.command_line.log_format.clone();
        let config_log = self.config.log.as_ref();
        let config_format = config_log.and_then(|logger| logger.format.clone());
        cmd_format.or(config_format).unwrap_or(LogFormat::Plain)
    }

    fn logger_output(&self) -> LogOutput {
        let cmd_output = self.command_line.log_output.clone();
        let config_log = self.config.log.as_ref();
        let config_output = config_log.and_then(|logger| logger.output.clone());
        cmd_output.or(config_output).unwrap_or(LogOutput::Stderr)
    }

    /// Load the settings
    /// - from the command arguments
    /// - from the config
    ///
    /// This function will print&exit if anything is not as it should be.
    pub fn try_into_settings(self, logger: &Logger) -> Result<Settings, Error> {
        let RawSettings {
            command_line,
            config,
        } = self;
        let command_arguments = &command_line.start_arguments;
        let network = generate_network(&command_arguments, &config);

        let storage = match (command_arguments.storage.as_ref(), config.storage) {
            (Some(path), _) => Some(path.clone()),
            (None, Some(path)) => Some(path.clone()),
            (None, None) => None,
        };

        let mut secrets = command_arguments.secret.clone();
        if let Some(secret_files) = config.secret_files {
            secrets.extend(secret_files);
        }

        if secrets.is_empty() {
            warn!(
                logger,
                "Node started without path to the stored secret keys"
            );
        };

        let block0_info = match (
            &command_arguments.block_0_path,
            &command_arguments.block_0_hash,
        ) {
            (None, None) => return Err(Error::ExpectedBlock0Info),
            (Some(_path), Some(_hash)) => return Err(Error::TooMuchBlock0Info),
            (Some(path), None) => Block0Info::Path(path.clone()),
            (None, Some(hash)) => Block0Info::Hash(hash.clone()),
        };

        let explorer = command_arguments.explorer_enabled
            || config.explorer.map_or(false, |settings| settings.enabled);

        Ok(Settings {
            storage: storage,
            block_0: block0_info,
            network: network,
            secrets,
            rest: config.rest,
            mempool: config.mempool,
            leadership: config.leadership,
            explorer,
        })
    }
}

fn generate_network(
    _command_arguments: &StartArguments,
    config: &Config,
) -> network::Configuration {
    let p2p = &config.p2p;
    network::Configuration {
        public_id: p2p.public_id.clone(),
        public_address: Some(p2p.public_address.clone()),
        listen_address: p2p
            .listen_address
            .clone()
            .and_then(|addr| addr.to_socketaddr()),
        trusted_peers: p2p.trusted_peers.clone().unwrap_or(vec![]),
        protocol: Protocol::Grpc,
        subscriptions: config
            .p2p
            .topics_of_interest
            .clone()
            .unwrap_or(BTreeMap::new()),
        timeout: std::time::Duration::from_secs(15),
    }
}
