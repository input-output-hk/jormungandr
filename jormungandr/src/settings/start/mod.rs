pub mod config;
pub mod network;

use self::config::{Config, Leadership};
pub use self::config::{Cors, Rest, Tls};
use self::network::Protocol;
use crate::settings::logging::{LogFormat, LogOutput, LogSettings, LogSettingsEntry};
use crate::settings::{command_arguments::*, Block0Info};
use jormungandr_lib::interfaces::Mempool;
use slog::{FilterLevel, Logger};
use std::{fs::File, path::PathBuf};
use thiserror::Error;

const DEFAULT_FILTER_LEVEL: FilterLevel = FilterLevel::Info;
const DEFAULT_LOG_FORMAT: LogFormat = LogFormat::Plain;
const DEFAULT_LOG_OUTPUT: LogOutput = LogOutput::Stderr;
const DEFAULT_NO_BLOCKCHAIN_UPDATES_WARNING_INTERVAL: u64 = 1800; // 30 min

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot read the node configuration file: {0}")]
    ConfigIo(#[from] std::io::Error),
    #[error("Error while parsing the node configuration file: {0}")]
    Config(#[from] serde_yaml::Error),
    #[error("Cannot start the node without the information to retrieve the genesis block")]
    ExpectedBlock0Info,
    #[error("In the node configuration file, the `p2p.listen_address` value is not a valid address. Use format `/ip4/x.x.x.x/tcp/4920")]
    ListenAddressNotValid,
}

/// Overall Settings for node
pub struct Settings {
    pub network: network::Configuration,
    pub storage: Option<PathBuf>,
    pub block_0: Block0Info,
    pub secrets: Vec<PathBuf>,
    pub rest: Option<Rest>,
    pub mempool: Mempool,
    pub rewards_report_all: bool,
    pub leadership: Leadership,
    pub explorer: bool,
    pub no_blockchain_updates_warning_interval: std::time::Duration,
}

pub struct RawSettings {
    command_line: CommandLine,
    config: Option<Config>,
}

impl RawSettings {
    pub fn load(command_line: CommandLine) -> Result<Self, Error> {
        let config = if let Some(node_config) = &command_line.start_arguments.node_config {
            Some(serde_yaml::from_reader(File::open(node_config)?)?)
        } else {
            None
        };
        Ok(Self {
            command_line,
            config,
        })
    }

    pub fn log_settings(&self) -> LogSettings {
        let mut entries = Vec::new();

        if let Some(log) = self.config.as_ref().and_then(|cfg| cfg.log.as_ref()) {
            log.0.iter().for_each(|entry| {
                entries.push(LogSettingsEntry {
                    level: entry.level.clone().unwrap_or(DEFAULT_FILTER_LEVEL),
                    format: entry.format.clone().unwrap_or(DEFAULT_LOG_FORMAT),
                    output: entry.output.clone().unwrap_or(DEFAULT_LOG_OUTPUT),
                })
            });
        }

        let cmd_level = self.command_line.log_level.clone();
        let cmd_format = self.command_line.log_format.clone();
        let cmd_output = self.command_line.log_output.clone();

        if cmd_level.is_some() || cmd_format.is_some() || cmd_output.is_some() {
            entries.push(LogSettingsEntry {
                level: cmd_level.unwrap_or(DEFAULT_FILTER_LEVEL),
                format: cmd_format.unwrap_or(DEFAULT_LOG_FORMAT),
                output: cmd_output.unwrap_or(DEFAULT_LOG_OUTPUT),
            });
        }

        if entries.is_empty() {
            entries.push(LogSettingsEntry {
                level: DEFAULT_FILTER_LEVEL,
                format: DEFAULT_LOG_FORMAT,
                output: DEFAULT_LOG_OUTPUT,
            });
        }

        LogSettings(entries)
    }

    fn rest_config(&self) -> Option<Rest> {
        let cmd_listen_opt = self.command_line.rest_arguments.listen.clone();
        let config_rest_opt = self.config.as_ref().and_then(|cfg| cfg.rest.as_ref());
        match (config_rest_opt, cmd_listen_opt) {
            (Some(config_rest), Some(cmd_listen)) => Some(Rest {
                listen: cmd_listen,
                ..config_rest.clone()
            }),
            (Some(config_rest), None) => Some(config_rest.clone()),
            (None, Some(cmd_listen)) => Some(Rest {
                listen: cmd_listen,
                tls: None,
                cors: None,
            }),
            (None, None) => None,
        }
    }

    /// Load the settings
    /// - from the command arguments
    /// - from the config
    ///
    /// This function will print&exit if anything is not as it should be.
    pub fn try_into_settings(self, logger: &Logger) -> Result<Settings, Error> {
        let rest = self.rest_config();
        let RawSettings {
            command_line,
            config,
        } = self;
        let command_arguments = &command_line.start_arguments;
        let network = generate_network(&command_arguments, &config, &logger)?;

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

        let block_0 = match (
            &command_arguments.block_0_path,
            &command_arguments.block_0_hash,
        ) {
            (None, None) => return Err(Error::ExpectedBlock0Info),
            (Some(path), Some(hash)) => Block0Info::Path(path.clone(), Some(hash.clone())),
            (Some(path), None) => Block0Info::Path(path.clone(), None),
            (None, Some(hash)) => Block0Info::Hash(hash.clone()),
        };

        let explorer = command_arguments.explorer_enabled
            || config.as_ref().map_or(false, |cfg| {
                cfg.explorer
                    .as_ref()
                    .map_or(false, |settings| settings.enabled)
            });

        Ok(Settings {
            storage,
            block_0,
            network,
            secrets,
            rewards_report_all: command_line.rewards_report_all,
            rest,
            mempool: config
                .as_ref()
                .map_or(Mempool::default(), |cfg| cfg.mempool.clone()),
            leadership: config
                .as_ref()
                .map_or(Leadership::default(), |cfg| cfg.leadership.clone()),
            explorer,
            no_blockchain_updates_warning_interval: config
                .as_ref()
                .and_then(|config| config.no_blockchain_updates_warning_interval.clone())
                .map(|d| d.into())
                .unwrap_or(std::time::Duration::from_secs(
                    DEFAULT_NO_BLOCKCHAIN_UPDATES_WARNING_INTERVAL,
                )),
        })
    }
}

fn generate_network(
    command_arguments: &StartArguments,
    config: &Option<Config>,
    logger: &Logger,
) -> Result<network::Configuration, Error> {
    let (mut p2p, http_fetch_block0_service, skip_bootstrap, bootstrap_from_trusted_peers) =
        if let Some(cfg) = config {
            (
                cfg.p2p.clone(),
                cfg.http_fetch_block0_service.clone(),
                cfg.skip_bootstrap.unwrap_or(false),
                cfg.bootstrap_from_trusted_peers.unwrap_or(false),
            )
        } else {
            (config::P2pConfig::default(), Vec::new(), false, false)
        };

    if p2p.trusted_peers.is_some() {
        p2p.trusted_peers
            .as_mut()
            .map(|peers| peers.extend(command_arguments.trusted_peer.clone()));
    } else if !command_arguments.trusted_peer.is_empty() {
        p2p.trusted_peers = Some(command_arguments.trusted_peer.clone())
    }

    let mut profile = poldercast::NodeProfileBuilder::new();

    if let Some(address) = p2p.public_address {
        profile.address(address.clone());
    }

    for (topic, interest_level) in p2p
        .topics_of_interest
        .unwrap_or(config::default_interests())
    {
        let sub = poldercast::Subscription {
            topic: topic.0,
            interest: interest_level.0,
        };
        profile.add_subscription(sub);
    }

    let mut network = network::Configuration {
        profile: profile.build(),
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
        trusted_peers: p2p
            .trusted_peers
            .clone()
            .unwrap_or(vec![])
            .into_iter()
            .map(Into::into)
            .collect(),
        protocol: Protocol::Grpc,
        policy: p2p.policy.clone(),
        layers: p2p.layers.clone(),
        max_connections: p2p
            .max_connections
            .unwrap_or(network::DEFAULT_MAX_CONNECTIONS),
        max_inbound_connections: p2p
            .max_inbound_connections
            .unwrap_or(network::DEFAULT_MAX_INBOUND_CONNECTIONS),
        timeout: std::time::Duration::from_secs(15),
        allow_private_addresses: p2p.allow_private_addresses,
        max_unreachable_nodes_to_connect_per_event: p2p.max_unreachable_nodes_to_connect_per_event,
        gossip_interval: p2p
            .gossip_interval
            .map(|d| d.into())
            .unwrap_or(std::time::Duration::from_secs(10)),
        topology_force_reset_interval: p2p.topology_force_reset_interval.map(|d| d.into()),
        max_bootstrap_attempts: p2p.max_bootstrap_attempts,
        http_fetch_block0_service,
        bootstrap_from_trusted_peers,
        skip_bootstrap,
    };

    if network.max_inbound_connections > network.max_connections {
        warn!(
            logger,
            "p2p.max_inbound_connections is larger than p2p.max_connections, decreasing from {} to {}",
            network.max_inbound_connections,
            network.max_connections
        );
        network.max_inbound_connections = network.max_connections;
    }

    Ok(network)
}
