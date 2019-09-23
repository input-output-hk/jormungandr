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
   ListenAddressNotValid = "In the node configuration file, the `p2p.listen_address` value is not a valid address. Use format `/ip4/x.x.x.x/tcp/4920",
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
    config: Option<Config>,
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
        let config_log = self.config.as_ref().and_then(|cfg| cfg.log.as_ref());
        let config_level = config_log.and_then(|log| log.level.clone());
        cmd_level.or(config_level).unwrap_or(FilterLevel::Info)
    }

    fn logger_format(&self) -> LogFormat {
        let cmd_format = self.command_line.log_format.clone();
        let config_log = self.config.as_ref().and_then(|cfg| cfg.log.as_ref());
        let config_format = config_log.and_then(|logger| logger.format.clone());
        cmd_format.or(config_format).unwrap_or(LogFormat::Plain)
    }

    fn logger_output(&self) -> LogOutput {
        let cmd_output = self.command_line.log_output.clone();
        let config_log = self.config.as_ref().and_then(|cfg| cfg.log.as_ref());
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
        let network = generate_network(&command_arguments, &config)?;

        let storage = match (
            command_arguments.storage.as_ref(),
            config.as_ref().map_or(None, |cfg| cfg.storage.as_ref()),
        ) {
            (Some(path), _) => Some(path.clone()),
            (None, Some(path)) => Some(path.clone()),
            (None, None) => None,
        };

        let mut secrets = command_arguments.secret.clone();
        if let Some(secret_files) = config.as_ref().map(|cfg| cfg.secret_files.clone()) {
            secrets.extend(secret_files);
        }

        if secrets.is_empty() {
            warn!(
                logger,
                "Node started without path to the stored secret keys (not a stake pool or a BFT leader)"
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
            || config.as_ref().map_or(false, |cfg| {
                cfg.explorer
                    .as_ref()
                    .map_or(false, |settings| settings.enabled)
            });

        Ok(Settings {
            storage: storage,
            block_0: block0_info,
            network: network,
            secrets,
            rest: config.as_ref().map_or(None, |cfg| cfg.rest.clone()),
            mempool: config
                .as_ref()
                .map_or(Mempool::default(), |cfg| cfg.mempool.clone()),
            leadership: config
                .as_ref()
                .map_or(Leadership::default(), |cfg| cfg.leadership.clone()),
            explorer,
        })
    }
}

fn generate_network(
    command_arguments: &StartArguments,
    config: &Option<Config>,
) -> Result<network::Configuration, Error> {
    let mut p2p = if let Some(cfg) = config {
        cfg.p2p.clone()
    } else {
        config::P2pConfig {
            public_address: None,
            listen_address: None,
            trusted_peers: None,
            topics_of_interest: None,
        }
    };

    if p2p.trusted_peers.is_some() {
        p2p.trusted_peers
            .as_mut()
            .map(|peers| peers.extend(command_arguments.trusted_peer.clone()));
    } else if !command_arguments.trusted_peer.is_empty() {
        p2p.trusted_peers = Some(command_arguments.trusted_peer.clone())
    }

    let network = network::Configuration {
        public_address: p2p.public_address.clone(),
        listen_address: match &p2p.listen_address {
            None => None,
            Some(v) => {
                if let Some(addr) = v.to_socketaddr() {
                    Some(addr)
                } else {
                    return Err(Error::ListenAddressNotValid);
                }
            }
        },
        trusted_peers: p2p.trusted_peers.clone().unwrap_or(vec![]),
        protocol: Protocol::Grpc,
        subscriptions: p2p.topics_of_interest.clone().unwrap_or(BTreeMap::new()),
        timeout: std::time::Duration::from_secs(15),
    };

    Ok(network)
}
