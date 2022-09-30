use crate::{
    blockcfg::HeaderHash,
    settings::{
        logging::{LogFormat, LogOutput},
        start::config::TrustedPeer,
        LOG_FILTER_LEVEL_POSSIBLE_VALUES,
    },
};
use multiaddr::Multiaddr;
use std::{net::SocketAddr, path::PathBuf};
use structopt::StructOpt;
use tracing::level_filters::LevelFilter;

fn trusted_peer_from_json(json: &str) -> Result<TrustedPeer, serde_json::Error> {
    serde_json::from_str(json)
}

#[derive(StructOpt, Debug)]
pub struct StartArguments {
    /// Path to the blockchain pool storage directory
    #[structopt(long = "storage", parse(from_os_str))]
    pub storage: Option<PathBuf>,

    /// Set the node config (in YAML format) to use as general configuration
    #[structopt(long = "config", parse(from_os_str))]
    pub node_config: Option<PathBuf>,

    /// Set the secret node config (in YAML format).
    #[structopt(long = "secret", parse(from_os_str))]
    pub secret: Option<PathBuf>,

    /// Path to the genesis block (the block0) of the blockchain
    #[structopt(long = "genesis-block", parse(try_from_str))]
    pub block_0_path: Option<PathBuf>,

    /// set a trusted peer in the multiformat format (e.g.: '/ip4/192.168.0.1/tcp/8029')
    ///
    /// This is the trusted peer the node will connect to initially to download the initial
    /// block0 and fast fetch missing blocks since last start of the node.
    #[structopt(long = "trusted-peer", parse(try_from_str = trusted_peer_from_json))]
    pub trusted_peer: Vec<TrustedPeer>,

    /// set the genesis block hash (the hash of the block0) so we can retrieve the
    /// genesis block (and the blockchain configuration) from the existing storage
    /// or from the network.
    #[structopt(long = "genesis-block-hash", parse(try_from_str))]
    pub block_0_hash: Option<HeaderHash>,

    /// Enable the Prometheus metrics exporter.
    #[cfg(feature = "prometheus-metrics")]
    #[structopt(long = "enable-prometheus")]
    pub prometheus_enabled: bool,

    /// The address to listen from and accept connection from. This is the
    /// public address that will be distributed to other peers of the network.
    #[structopt(long = "public-address")]
    pub public_address: Option<Multiaddr>,

    /// Specifies the address the node will listen to to receive p2p connection.
    /// Can be left empty and the node will listen to whatever value was given
    /// to `public_address`.
    #[structopt(long = "listen-address")]
    pub listen_address: Option<SocketAddr>,
}

#[derive(StructOpt, Debug)]
pub struct RestArguments {
    /// REST API listening address.
    /// If not configured anywhere, defaults to REST API being disabled
    #[structopt(name = "rest-listen")]
    pub listen: Option<SocketAddr>,
}

#[derive(StructOpt, Debug)]
pub struct JRpcArguments {
    /// JRPC API listening address.
    /// If not configured anywhere, defaults to JRPC API being disabled
    #[structopt(name = "jrpc-listen")]
    pub listen: Option<SocketAddr>,
}

#[derive(StructOpt, Debug)]
#[structopt(
    name = "jormungandr",
    setting = structopt::clap::AppSettings::ColoredHelp
)]
pub struct CommandLine {
    /// Set log messages minimum severity. If not configured anywhere, defaults to "info".
    #[structopt(
        long = "log-level",
        parse(try_from_str = log_level_parse),
        possible_values = &LOG_FILTER_LEVEL_POSSIBLE_VALUES
    )]
    pub log_level: Option<LevelFilter>,

    /// Set format of the log emitted. Can be "json" or "plain".
    /// If not configured anywhere, defaults to "plain".
    #[structopt(long = "log-format", parse(try_from_str))]
    pub log_format: Option<LogFormat>,

    /// Set format of the log emitted. Can be "stdout", "stderr",
    /// "syslog" (Unix only) or "journald"
    /// (linux with systemd only, must be enabled during compilation).
    /// If not configured anywhere, defaults to "stderr".
    #[structopt(long = "log-output", parse(try_from_str))]
    pub log_output: Option<LogOutput>,

    /// report all the rewards in the reward distribution history
    ///
    /// NOTE: this will slowdown the epoch transition computation and will add
    /// add a lot of items for in-memory operations, this is not recommended to set
    #[structopt(long = "rewards-report-all")]
    pub rewards_report_all: bool,

    #[structopt(flatten)]
    pub rest_arguments: RestArguments,

    #[structopt(flatten)]
    pub jrpc_arguments: JRpcArguments,

    #[structopt(flatten)]
    pub start_arguments: StartArguments,

    /// display full version details (software version, source version, targets and compiler used)
    #[structopt(long = "full-version")]
    pub full_version: bool,

    /// display the sources version, allowing to check the source's hash used to compile this executable.
    /// this option is useful for scripting retrieving the logs of the version of this application.
    #[structopt(long = "source-version")]
    pub source_version: bool,

    /// Initialize the storage and exit, useful to check that the storage has been set up correctly.
    #[structopt(long = "storage-check")]
    pub storage_check: bool,
}

impl CommandLine {
    /// load the command arguments from the command line args
    ///
    /// on error during reading the command line arguments, the
    /// function will print an error message and will terminate
    /// the process.
    ///
    pub fn load() -> Self {
        Self::from_args()
    }
}

fn log_level_parse(level: &str) -> Result<LevelFilter, String> {
    level
        .parse()
        .map_err(|_| format!("Unknown log level value: '{}'", level))
}
