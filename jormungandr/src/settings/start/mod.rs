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
        LogSettings {
            level: self.logger_level(),
            format: self.logger_format(),
            output: self.logger_output(),
        }
        .to_logger()
    }

    fn logger_level(&self) -> slog::Level {
        // Determine a logging level from the CLI arguments:
        let command_q = match self.command_line.quietness {
            0 => None,
            level => Some(level),
        };
        let command_v = match self.command_line.verbosity {
            0 => None,
            level => Some(level),
        };
        let command_level = determine_log_level(command_q, command_v).expect(
            "The following CLI arguments are mutually exclusive: \
             '--quiet', '--verbose'.",
        );
        // Determine a logging level from the configuration file:
        let config_logger = self.config.logger.as_ref();
        let config_q = config_logger.and_then(|l| l.quietness.clone());
        let config_v = config_logger.and_then(|l| l.verbosity.clone());
        let config_level = determine_log_level(config_q, config_v).expect(
            "The following configuration options are mutually exclusive: \
             'quietness', 'verbosity'.",
        );
        // Select a logging level according to an order of precedence:
        command_level.or(config_level).unwrap_or(DEFAULT_LOG_LEVEL)
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

/// The default log level to use when none could be determined from CLI
/// arguments or the configuration.
const DEFAULT_LOG_LEVEL: slog::Level = slog::Level::Info;

/// Determine a log level from *either* a quietness level *or* a verbosity
/// level.
///
/// If *neither* a quietness level *nor* a verbosity level are specified, this
/// function returns `Ok(None)`.
///
/// If *both* a quietness level *and* a verbosity level are specified, this
/// function returns an error.
///
fn determine_log_level(
    quietness_level: Option<u8>,
    verbosity_level: Option<u8>,
) -> Result<Option<slog::Level>, ()> {
    match (quietness_level, verbosity_level) {
        (None, None) => Ok(None),
        (Some(1), None) => Ok(Some(slog::Level::Warning)),
        (Some(2), None) => Ok(Some(slog::Level::Error)),
        (Some(_), None) => Ok(Some(slog::Level::Critical)),
        (None, Some(1)) => Ok(Some(slog::Level::Debug)),
        (None, Some(_)) => Ok(Some(slog::Level::Trace)),
        _ => Err(()),
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
