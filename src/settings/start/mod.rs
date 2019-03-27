mod config;
pub mod network;

use self::config::ConfigLogSettings;
pub use self::config::{Genesis, GenesisConstants, Rest};
use self::network::Protocol;
use crate::rest::Error as RestError;
use crate::settings::command_arguments::*;
use crate::settings::logging::LogSettings;

use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    fs::File,
    path::PathBuf,
};

#[derive(Debug)]
pub enum Error {
    Config(serde_yaml::Error),
    NoConsensusAlg,
    NoStorage,
    NoSecret,
    InvalidRest(RestError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Config(e) => write!(f, "config error: {}", e),
            Error::NoConsensusAlg => write!(f, "no consensus algorithm defined"),
            Error::NoStorage => write!(
                f,
                "storage is needed for persistently saving the blocks of the blockchain"
            ),
            Error::NoSecret => write!(f, "secret config unspecified"),
            Error::InvalidRest(e) => write!(f, "invalid REST config: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            Error::Config(e) => Some(e),
            Error::NoConsensusAlg => None,
            Error::NoStorage => None,
            Error::NoSecret => None,
            Error::InvalidRest(e) => Some(e),
        }
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Error {
        Error::Config(e)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leadership {
    Yes,
    No,
}

impl From<bool> for Leadership {
    fn from(b: bool) -> Self {
        if b {
            Leadership::Yes
        } else {
            Leadership::No
        }
    }
}

/// Overall Settings for node
pub struct Settings {
    pub network: network::Configuration,

    pub storage: Option<PathBuf>,

    pub block_0: PathBuf,

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
    pub fn load(
        command_line: &CommandLine,
        command_arguments: &StartArguments,
    ) -> Result<Self, Error> {
        let config: config::Config = {
            let mut file = File::open(command_arguments.node_config.clone()).unwrap();
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
                (None, None) => return Err(Error::NoSecret),
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

    pub fn load_block_0(&self) -> crate::blockcfg::Block {
        use chain_core::property::Deserialize as _;
        let f = File::open(&self.block_0).unwrap();
        let reader = std::io::BufReader::new(f);
        crate::blockcfg::Block::deserialize(reader).unwrap()
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
            0 => slog::Level::Warning,
            1 => slog::Level::Info,
            2 => slog::Level::Debug,
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
