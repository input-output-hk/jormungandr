mod config;
pub mod network;

pub use self::config::Rest;
use self::config::{Config, ConfigLogSettings};
use self::network::Protocol;
use crate::rest::Error as RestError;
use crate::settings::logging::{self, LogFormat, LogOutput, LogSettings};
use crate::settings::{command_arguments::*, Block0Info};
use slog::Logger;

use std::{collections::BTreeMap, fs::File, path::PathBuf};

custom_error! {pub Error
   ConfigIo { source: std::io::Error } = "Cannot read the node configuration file: {source}",
   Config { source: serde_yaml::Error } = "Error while parsing the node configuration file: {source}",
   Rest { source: RestError } = "The Rest configuration is invalid: {source}",
   ExpectedBlock0Info = "Cannot start the node without the information to retrieve the genesis block",
   TooMuchBlock0Info = "Use only `--genesis-block-hash' or `--genesis-block'",
}

/// Overall Settings for node
pub struct Settings {
    pub network: network::Configuration,
    pub storage: Option<PathBuf>,
    pub block_0: Block0Info,
    pub leadership: Vec<PathBuf>,
    pub rest: Option<Rest>,
}

pub struct RawSettings {
    command_line: CommandLine,
    config: Config,
}

impl RawSettings {
    pub fn load(command_line: CommandLine) -> Result<Self, Error> {
        let config_file = File::open(&command_line.start_arguments.node_config)?;
        let config = serde_yaml::from_reader(config_file)?;
        Ok(Self {
            command_line,
            config,
        })
    }

    pub fn to_logger(&self) -> Result<Logger, logging::Error> {
        let backend = self.config.logger.as_ref().and_then(|l| l.backend.clone());
        let logs_id = self.config.logger.as_ref().and_then(|l| l.logs_id.clone());
        LogSettings {
            verbosity: self.logger_verbosity(),
            format: self.logger_format(),
            output: self.logger_output(),
            backend,
            logs_id,
        }
        .to_logger()
    }

    fn logger_verbosity(&self) -> slog::Level {
        let cmd_level = match self.command_line.verbose {
            0 => None,
            level => Some(level),
        };
        let config_logger = self.config.logger.as_ref();
        let config_level = config_logger.and_then(|logger| logger.verbosity.clone());
        let level = cmd_level.or(config_level).unwrap_or(0);
        match level {
            0 => slog::Level::Info,
            1 => slog::Level::Debug,
            _ => slog::Level::Trace,
        }
    }

    fn logger_format(&self) -> LogFormat {
        let cmd_format = self.command_line.log_format.clone();
        let config_logger = self.config.logger.as_ref();
        let config_format = config_logger.and_then(|logger| logger.format.clone());
        cmd_format.or(config_format).unwrap_or(LogFormat::Plain)
    }

    fn logger_output(&self) -> LogOutput {
        let cmd_output = self.command_line.log_output.clone();
        let config_logger = self.config.logger.as_ref();
        let config_output = config_logger.and_then(|logger| logger.output.clone());
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

        let mut leadership = command_arguments.secret.clone();
        if let Some(secret_files) = config.secret_files {
            leadership.extend(secret_files);
        }

        if leadership.is_empty() {
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

        Ok(Settings {
            storage: storage,
            block_0: block0_info,
            network: network,
            leadership,
            rest: config.rest,
        })
    }
}

fn generate_network(
    _command_arguments: &StartArguments,
    config: &Config,
) -> network::Configuration {
    let p2p = &config.peer_2_peer;
    network::Configuration {
        public_id: p2p.public_id.clone(),
        public_address: p2p.public_address.clone(),
        listen: p2p.listen.clone(),
        trusted_peers: p2p.trusted_peers.clone().unwrap_or(vec![]),
        protocol: Protocol::Grpc,
        subscriptions: config
            .peer_2_peer
            .topics_of_interests
            .clone()
            .unwrap_or(BTreeMap::new()),
        timeout: std::time::Duration::from_secs(15),
    }
}
