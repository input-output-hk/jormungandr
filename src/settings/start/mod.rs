mod config;
pub mod network;

use self::config::ConfigLogSettings;
pub use self::config::{Genesis, GenesisConstants, Rest};
use self::network::{Connection, Listen, Peer, Protocol};
use crate::blockcfg::genesis_data::*;
use crate::rest::Error as RestError;
use crate::settings::command_arguments::*;
use crate::settings::logging::LogSettings;

use std::{
    collections::HashMap,
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

    pub storage: PathBuf,

    pub genesis_data_config: PathBuf,

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
            (Some(path), _) => path.clone(),
            (None, Some(path)) => path.clone(),
            (None, None) => return Err(Error::NoStorage),
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
            genesis_data_config: command_arguments.genesis_data_config.clone(),
            network: network,
            leadership: secret,
            log_settings: log_settings,
            rest: config.rest,
        })
    }

    pub fn read_genesis_data(&self) -> Result<GenesisData, impl std::error::Error> {
        let f = File::open(&self.genesis_data_config).unwrap();
        let mut reader = std::io::BufReader::new(f);

        GenesisData::parse(&mut reader)
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
    command_arguments: &StartArguments,
    config: &config::Config,
) -> network::Configuration {
    let mut peer_nodes_map: HashMap<_, _> =
        config
            .legacy_peers
            .as_ref()
            .map_or(HashMap::new(), |addresses| {
                addresses
                    .iter()
                    .cloned()
                    .map(|addr| (addr, Protocol::Ntt))
                    .collect()
            });
    peer_nodes_map.extend(
        config
            .grpc_peers
            .as_ref()
            .map_or(HashMap::new(), |addresses| {
                addresses
                    .iter()
                    .cloned()
                    .map(|addr| (addr, Protocol::Grpc))
                    .collect()
            }),
    );
    peer_nodes_map.extend(
        command_arguments
            .ntt_connect
            .iter()
            .cloned()
            .map(|addr| (addr, Protocol::Ntt)),
    );
    peer_nodes_map.extend(
        command_arguments
            .grpc_connect
            .iter()
            .cloned()
            .map(|addr| (addr, Protocol::Grpc)),
    );
    let peer_nodes = peer_nodes_map
        .iter()
        .map(|(&addr, proto)| Peer::new(Connection::Tcp(addr), proto.clone()))
        .collect();

    let mut listen_map: HashMap<_, _> =
        config
            .legacy_listen
            .as_ref()
            .map_or(HashMap::new(), |addresses| {
                addresses
                    .iter()
                    .cloned()
                    .map(|addr| (addr, Protocol::Ntt))
                    .collect()
            });
    if let Some(addresses) = config.grpc_listen.as_ref() {
        listen_map.extend(addresses.iter().cloned().map(|addr| (addr, Protocol::Grpc)));
    };
    listen_map.extend(
        command_arguments
            .ntt_listen
            .iter()
            .cloned()
            .map(|addr| (addr, Protocol::Ntt)),
    );
    listen_map.extend(
        command_arguments
            .grpc_listen
            .iter()
            .cloned()
            .map(|addr| (addr, Protocol::Grpc)),
    );
    let listen_to: Vec<_> = listen_map
        .iter()
        .map(|(&addr, proto)| Listen::new(Connection::Tcp(addr), proto.clone()))
        .collect();

    network::Configuration {
        peer_nodes,
        listen_to,
    }
}
