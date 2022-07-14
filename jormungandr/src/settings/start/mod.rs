pub mod config;
pub mod network;

use self::{
    config::{Config, Leadership},
    network::{Protocol, TrustedPeer},
};
use crate::{
    settings::{
        command_arguments::*,
        logging::{LogFormat, LogInfoMsg, LogOutput, LogSettings, LogSettingsEntry},
        Block0Info,
    },
    topology::layers::{self, LayersConfig, PreferredListConfig, RingsConfig},
};
use chain_crypto::Ed25519;
pub use jormungandr_lib::interfaces::{Cors, JRpc, Mempool, Rest, Tls};
use jormungandr_lib::{crypto::key::SigningKey, multiaddr};
use std::{convert::TryFrom, fs::File, path::PathBuf};
use thiserror::Error;
use tracing::level_filters::LevelFilter;

const DEFAULT_FILTER_LEVEL: LevelFilter = LevelFilter::TRACE;
const DEFAULT_LOG_FORMAT: LogFormat = LogFormat::Default;
const DEFAULT_LOG_OUTPUT: LogOutput = LogOutput::Stderr;
const DEFAULT_NO_BLOCKCHAIN_UPDATES_WARNING_INTERVAL: u64 = 1800; // 30 min
const DEFAULT_BLOCK_HARD_DEADLINE: u32 = 50;
const DEFAULT_LOG_SETTINGS_ENTRY: LogSettingsEntry = LogSettingsEntry {
    level: DEFAULT_FILTER_LEVEL,
    format: DEFAULT_LOG_FORMAT,
    output: DEFAULT_LOG_OUTPUT,
};

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
    #[error(transparent)]
    InvalidLayersConfig(#[from] layers::ParseError),
}

/// Overall Settings for node
pub struct Settings {
    pub network: network::Configuration,
    pub storage: Option<PathBuf>,
    pub block_0: Block0Info,
    pub secret: Option<PathBuf>,
    pub rest: Option<Rest>,
    pub jrpc: Option<JRpc>,
    pub mempool: Mempool,
    pub rewards_report_all: bool,
    pub leadership: Leadership,
    #[cfg(feature = "prometheus-metrics")]
    pub prometheus: bool,
    pub no_blockchain_updates_warning_interval: std::time::Duration,
    pub block_hard_deadline: u32,
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
        // Start with default config
        let mut log_config = DEFAULT_LOG_SETTINGS_ENTRY;
        let mut info_msgs: Vec<String> = Vec::new();

        //  Read log settings from the config file path.
        if let Some(cfg) = self.config.as_ref().and_then(|cfg| cfg.log.as_ref()) {
            if let Some(level) = cfg.level {
                log_config.level = level;
            }
            if let Some(format) = cfg.format {
                log_config.format = format;
            }
            if let Some(output) = &cfg.output {
                log_config.output = output.clone();
            }
        }

        // If the command line specifies log arguments, they override everything
        // else.
        if let Some(output) = &self.command_line.log_output {
            if &log_config.output != output {
                info_msgs.push(format!(
                    "log output overriden from command line: {:?} replaced with {:?}",
                    log_config.output, output
                ));
            }
            log_config.output = output.clone();
        }
        if let Some(level) = self.command_line.log_level {
            if log_config.level != level {
                info_msgs.push(format!(
                    "log level overriden from command line: {:?} replaced with {:?}",
                    log_config.level, level
                ));
            }
            log_config.level = level;
        }
        if let Some(format) = self.command_line.log_format {
            if log_config.format != format {
                info_msgs.push(format!(
                    "log format overriden from command line: {:?} replaced with {:?}",
                    log_config.format, format
                ));
            }
            log_config.format = format;
        }

        let log_info_msg: LogInfoMsg = if info_msgs.is_empty() {
            None
        } else {
            Some(info_msgs)
        };
        LogSettings {
            config: log_config,
            msgs: log_info_msg,
        }
    }

    fn rest_config(&self) -> Option<Rest> {
        let cmd_listen_opt = self.command_line.rest_arguments.listen;
        let config_rest_opt = self.config.as_ref().and_then(|cfg| cfg.rest.clone());
        match (config_rest_opt, cmd_listen_opt) {
            (Some(config_rest), Some(cmd_listen)) => Some(Rest {
                listen: cmd_listen,
                ..config_rest
            }),
            (Some(config_rest), None) => Some(config_rest),
            (None, Some(cmd_listen)) => Some(Rest {
                listen: cmd_listen,
                tls: None,
                cors: None,
            }),
            (None, None) => None,
        }
    }

    fn jrpc_config(&self) -> Option<JRpc> {
        let cmd_listen_opt = self.command_line.jrpc_arguments.listen;
        let config_rpc_opt = self.config.as_ref().and_then(|cfg| cfg.jrpc.clone());
        match (config_rpc_opt, cmd_listen_opt) {
            (Some(_), Some(cmd_listen)) => Some(JRpc { listen: cmd_listen }),
            (Some(config_rpc), None) => Some(config_rpc),
            (None, Some(cmd_listen)) => Some(JRpc { listen: cmd_listen }),
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
        let jrpc = self.jrpc_config();
        let RawSettings {
            command_line,
            config,
        } = self;
        let command_arguments = &command_line.start_arguments;
        let network = generate_network(command_arguments, &config)?;

        let storage = match (
            command_arguments.storage.as_ref(),
            config.as_ref().and_then(|cfg| cfg.storage.as_ref()),
        ) {
            (Some(path), _) => Some(path.clone()),
            (None, Some(path)) => Some(path.clone()),
            (None, None) => None,
        };

        let secret = command_arguments
            .secret
            .clone()
            .or_else(|| config.as_ref().and_then(|cfg| cfg.secret_file.clone()));
        if secret.is_none() {
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

        #[cfg(feature = "prometheus-metrics")]
        let prometheus = command_arguments.prometheus_enabled
            || config.as_ref().map_or(false, |cfg| {
                cfg.prometheus
                    .as_ref()
                    .map_or(false, |settings| settings.enabled)
            });

        Ok(Settings {
            storage,
            block_0,
            network,
            secret,
            rewards_report_all: command_line.rewards_report_all,
            rest,
            jrpc,
            mempool: config
                .as_ref()
                .map_or(Mempool::default(), |cfg| cfg.mempool.clone()),
            leadership: config
                .as_ref()
                .map_or(Leadership::default(), |cfg| cfg.leadership.clone()),
            #[cfg(feature = "prometheus-metrics")]
            prometheus,
            no_blockchain_updates_warning_interval: config
                .as_ref()
                .and_then(|config| config.no_blockchain_updates_warning_interval)
                .map(|d| d.into())
                .unwrap_or_else(|| {
                    std::time::Duration::from_secs(DEFAULT_NO_BLOCKCHAIN_UPDATES_WARNING_INTERVAL)
                }),
            block_hard_deadline: config
                .as_ref()
                .and_then(|config| config.block_hard_deadline)
                .unwrap_or(DEFAULT_BLOCK_HARD_DEADLINE),
        })
    }
}

fn resolve_trusted_peers(peers: &[jormungandr_lib::interfaces::TrustedPeer]) -> Vec<TrustedPeer> {
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

    let trusted_peers = p2p
        .trusted_peers
        .as_ref()
        .map_or_else(Vec::new, |peers| resolve_trusted_peers(peers));

    // Layers config
    let preferred_list_config = p2p.layers.preferred_list.unwrap_or_default();
    let preferred_list = PreferredListConfig {
        view_max: preferred_list_config.view_max.into(),
        peers: resolve_trusted_peers(&preferred_list_config.peers),
    };
    let rings = p2p
        .layers
        .topics_of_interest
        .map(RingsConfig::try_from)
        .transpose()?
        .unwrap_or_default();

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
        protocol: Protocol::Grpc,
        layers: LayersConfig {
            preferred_list,
            rings,
        },
        max_connections: p2p
            .max_connections
            .unwrap_or(network::DEFAULT_MAX_CONNECTIONS),
        max_client_connections: p2p
            .max_client_connections
            .unwrap_or(network::DEFAULT_MAX_CLIENT_CONNECTIONS),
        timeout: std::time::Duration::from_secs(15),
        allow_private_addresses: p2p.allow_private_addresses,
        gossip_interval: p2p
            .gossip_interval
            .map(|d| d.into())
            .unwrap_or_else(|| std::time::Duration::from_secs(10)),
        network_stuck_check: p2p
            .network_stuck_check
            .map(Into::into)
            .unwrap_or(crate::topology::DEFAULT_NETWORK_STUCK_INTERVAL),
        max_bootstrap_attempts: p2p.max_bootstrap_attempts,
        http_fetch_block0_service,
        bootstrap_from_trusted_peers,
        skip_bootstrap,
    };

    if network.max_client_connections > network.max_connections {
        tracing::warn!(
            "p2p.max_client_connections is larger than p2p.max_connections, decreasing from {} to {}",
            network.max_client_connections,
            network.max_connections
        );
        network.max_client_connections = network.max_connections;
    }

    Ok(network)
}
