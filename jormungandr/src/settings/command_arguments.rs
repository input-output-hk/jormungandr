use crate::settings::LOG_FILTER_LEVEL_POSSIBLE_VALUES;
use slog::FilterLevel;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::{
    blockcfg::HeaderHash,
    settings::logging::{LogFormat, LogOutput},
};

#[derive(StructOpt, Debug)]
pub struct StartArguments {
    /// Path to the blockchain pool storage directory
    #[structopt(long = "storage", parse(from_os_str))]
    pub storage: Option<PathBuf>,

    /// Set the node config (in YAML format) to use as general configuration
    #[structopt(long = "config", parse(from_os_str))]
    pub node_config: PathBuf,

    /// Set the secret node config (in YAML format). Can be given
    /// multiple times.
    #[structopt(long = "secret", parse(from_os_str))]
    pub secret: Vec<PathBuf>,

    /// Path to the genesis block (the block0) of the blockchain
    #[structopt(long = "genesis-block", parse(try_from_str))]
    pub block_0_path: Option<PathBuf>,

    /// set the genesis block hash (the hash of the block0) so we can retrieve the
    /// genesis block (and the blockchain configuration) from the existing storage
    /// or from the network.
    #[structopt(long = "genesis-block-hash", parse(try_from_str))]
    pub block_0_hash: Option<HeaderHash>,

    /// Start the explorer task and enable associated query endpoints.
    #[structopt(long = "enable-explorer")]
    pub explorer_enabled: bool,
}

#[derive(StructOpt, Debug)]
#[structopt(
    name = "jormungandr",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
pub struct CommandLine {
    /// Set log messages minimum severity. If not configured anywhere, defaults to "info".
    #[structopt(
        long = "log-level",
        parse(try_from_str = "log_level_parse"),
        raw(possible_values = "&LOG_FILTER_LEVEL_POSSIBLE_VALUES")
    )]
    pub log_level: Option<FilterLevel>,

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

    #[structopt(flatten)]
    pub start_arguments: StartArguments,

    /// display full version details (software version, source version, targets and compiler used)
    #[structopt(long = "full-version")]
    pub full_version: bool,

    /// display the sources version, allowing to check the source's hash used to compile this executable.
    /// this option is useful for scripting retrieving the logs of the version of this application.
    #[structopt(long = "source-version")]
    pub source_version: bool,
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

fn log_level_parse(level: &str) -> Result<FilterLevel, String> {
    level
        .parse()
        .map_err(|_| format!("Unknown log level value: '{}'", level))
}
