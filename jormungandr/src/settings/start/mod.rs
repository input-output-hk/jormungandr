pub mod config;
pub mod network;

use self::config::{Config, Leadership};
use self::network::{Protocol, TrustedPeer};
use crate::settings::logging::{LogFormat, LogInfoMsg, LogOutput, LogSettings, LogSettingsEntry};
use crate::settings::{command_arguments::*, Block0Info};
use chain_crypto::Ed25519;
use jormungandr_lib::crypto::key::SigningKey;
pub use jormungandr_lib::interfaces::{Cors, Mempool, Rest, Tls};
use jormungandr_lib::multiaddr;
use std::{fs::File, path::PathBuf};
use thiserror::Error;
use tracing::level_filters::LevelFilter;

const DEFAULT_FILTER_LEVEL: LevelFilter = LevelFilter::TRACE;
const DEFAULT_LOG_FORMAT: LogFormat = LogFormat::Default;
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
    #[error(transparent)]
    InvalidMultiaddr(#[from] multiaddr::Error),
    #[error("cannot deserialize node key from file")]
    InvalidKey(#[from] chain_crypto::bech32::Error),
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
        let mut log_info_msg: LogInfoMsg = None;

        //  Read log settings from the config file path.
        if let Some(log) = self.config.as_ref().and_then(|cfg| cfg.log.as_ref()) {
            log.0.iter().for_each(|entry| {
                entries.push(LogSettingsEntry {
                    level: entry.level.clone().unwrap_or(DEFAULT_FILTER_LEVEL),
                    format: entry.format.clone().unwrap_or(DEFAULT_LOG_FORMAT),
                    output: entry.output.clone().unwrap_or(DEFAULT_LOG_OUTPUT),
                })
            });
        }

        // If the command line specifies log arguments, check that the arg
        // is Some(output), else, skip.
        let cmd_output = self.command_line.log_output.clone();

        if let Some(output) = cmd_output {
            let cmd_level = self.command_line.log_level;
            let cmd_format = self.command_line.log_format;

            let command_line_entry = LogSettingsEntry {
                level: cmd_level.unwrap_or(DEFAULT_FILTER_LEVEL),
                format: cmd_format.unwrap_or(DEFAULT_LOG_FORMAT),
                output,
            };
            // check if there are entries being overriden
            if !entries.is_empty() {
                // log to info! that the output was overriden,
                // we send this as a message because tracing Subscribers
                // do not get initiated until after this code runs
                log_info_msg = Some(format!(
                    "log settings overriden from command line: {:?} replaced with {:?}",
                    entries, command_line_entry
                ));
            }
            // Replace any existing setting entries with the
            // command line settings entry.
            entries = vec![command_line_entry];
        }

        //  If no log settings are found, add LogSettingsEntry with default
        //  values.
        if entries.is_empty() {
            entries.push(LogSettingsEntry {
                level: DEFAULT_FILTER_LEVEL,
                format: DEFAULT_LOG_FORMAT,
                output: DEFAULT_LOG_OUTPUT,
            });
        }

        LogSettings(entries, log_info_msg)
    }

    fn rest_config(&self) -> Option<Rest> {
        let cmd_listen_opt = self.command_line.rest_arguments.listen;
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
    pub fn try_into_settings(self) -> Result<Settings, Error> {
        let rest = self.rest_config();
        let RawSettings {
            command_line,
            config,
        } = self;
        let command_arguments = &command_line.start_arguments;
        let network = generate_network(&command_arguments, &config)?;

        let storage = match (
            command_arguments.storage.as_ref(),
            config.as_ref().and_then(|cfg| cfg.storage.as_ref()),
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
            tracing::warn!(
                "Node started without path to the stored secret keys (not a stake pool or a BFT leader)"
            );
        };

        let block_0 = match (
            &command_arguments.block_0_path,
            &command_arguments.block_0_hash,
        ) {
            (None, None) => return Err(Error::ExpectedBlock0Info),
            (Some(path), Some(hash)) => Block0Info::Path(path.clone(), Some(*hash)),
            (Some(path), None) => Block0Info::Path(path.clone(), None),
            (None, Some(hash)) => Block0Info::Hash(*hash),
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
                .and_then(|config| config.no_blockchain_updates_warning_interval)
                .map(|d| d.into())
                .unwrap_or_else(|| {
                    std::time::Duration::from_secs(DEFAULT_NO_BLOCKCHAIN_UPDATES_WARNING_INTERVAL)
                }),
        })
    }
}

#[allow(deprecated)]
fn generate_network(
    command_arguments: &StartArguments,
    config: &Option<Config>,
) -> Result<network::Configuration, Error> {
    let (mut p2p, http_fetch_block0_service, skip_bootstrap, bootstrap_from_trusted_peers) =
        if let Some(cfg) = config {
            (
                cfg.p2p.clone(),
                cfg.http_fetch_block0_service.clone(),
                cfg.skip_bootstrap,
                cfg.bootstrap_from_trusted_peers,
            )
        } else {
            (config::P2pConfig::default(), Vec::new(), false, false)
        };

    if p2p.trusted_peers.is_some() {
        if let Some(peers) = p2p.trusted_peers.as_mut() {
            peers.extend(command_arguments.trusted_peer.clone())
        }
    } else if !command_arguments.trusted_peer.is_empty() {
        p2p.trusted_peers = Some(command_arguments.trusted_peer.clone())
    }

    let trusted_peers = p2p.trusted_peers.as_ref().map_or_else(Vec::new, |peers| {
        peers
            .iter()
            .filter_map(|config_peer| match TrustedPeer::resolve(config_peer) {
                Ok(peer) => {
                    tracing::info!(
                        config = %config_peer.address,
                        resolved = %peer.addr,
                        "DNS resolved for trusted peer"
                    );
                    Some(peer)
                }
                Err(e) => {
                    tracing::warn!(
                        config = %config_peer.address,
                        reason = %e,
                        "failed to resolve trusted peer address"
                    );
                    None
                }
            })
            .collect()
    });

    // TODO: do we want to check that we end up with a valid address?
    // Is it possible for a node to specify no public address?
    let config_addr = p2p.public_address;
    let public_address = command_arguments
        .public_address
        .clone()
        .or(config_addr)
        .and_then(|addr| multiaddr::to_tcp_socket_addr(&addr));

    let node_key = match p2p.node_key_file {
        Some(node_key_file) => {
            <SigningKey<Ed25519>>::from_bech32_str(&std::fs::read_to_string(&node_key_file)?)?
        }
        None => SigningKey::generate(rand::thread_rng()),
    };

    let p2p_listen_address = p2p.listen.as_ref();
    let listen_address = command_arguments
        .listen_address
        .as_ref()
        .or(p2p_listen_address)
        .cloned();

    let mut network = network::Configuration {
        listen_address,
        public_address,
        trusted_peers,
        node_key,
        policy: p2p.policy.clone(),
        topics_of_interest: p2p
            .topics_of_interest
            .clone()
            .unwrap_or_else(config::default_interests),
        protocol: Protocol::Grpc,
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
            .unwrap_or_else(|| std::time::Duration::from_secs(10)),
        topology_force_reset_interval: p2p.topology_force_reset_interval.map(|d| d.into()),
        max_bootstrap_attempts: p2p.max_bootstrap_attempts,
        http_fetch_block0_service,
        bootstrap_from_trusted_peers,
        skip_bootstrap,
    };

    if network.max_inbound_connections > network.max_connections {
        tracing::warn!(
            "p2p.max_inbound_connections is larger than p2p.max_connections, decreasing from {} to {}",
            network.max_inbound_connections,
            network.max_connections
        );
        network.max_inbound_connections = network.max_connections;
    }

    Ok(network)
}
