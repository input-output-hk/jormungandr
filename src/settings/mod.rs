mod command_arguments;
mod config;
pub mod network;

use std::path::PathBuf;
use cardano::config::GenesisData;
use std::fs::File;
use std::io::Read;
use std;

use exe_common::parse_genesis_data::parse_genesis_data;

pub use self::command_arguments::CommandArguments;
pub use self::config::{Bft, BftConstants, Genesis, GenesisConstants, BftLeader};
use self::network::{Connection, Listen, Peer, Protocol};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Leadership {
    Yes,
    No,
}

impl From<bool> for Leadership {
    fn from(b: bool) -> Self { if b { Leadership::Yes } else { Leadership::No } }
}

/// Overall Settings for node
pub struct Settings {
    pub cmd_args: CommandArguments,

    pub network: network::Configuration,

    pub genesis_data_config: PathBuf,

    pub secret_config: PathBuf,

    pub consensus: Consensus,

    pub leadership: Leadership,
}

#[derive(Debug)]
pub enum Consensus {
    /// BFT consensus
    Bft(config::Bft),
    /// Genesis consensus
    Genesis,
}


impl Settings {
    /// Load the settings
    /// - from the command arguments
    /// - from the config
    ///
    /// This function will print&exit if anything is not as it should be.
    pub fn load() -> Self {
        let command_arguments = CommandArguments::load();

        let peer_nodes = command_arguments.connect_to.iter().cloned()
            .map(|addr| {
                Peer::new(Connection::Tcp(addr), Protocol::Ntt)
            }).collect();

        let mut listen_to: Vec<_> = command_arguments.ntt_listen.iter().cloned()
            .map(|addr| {
                Listen::new(Connection::Tcp(addr), Protocol::Ntt)
            }).collect();
        listen_to.extend(command_arguments.grpc_listen.iter().cloned()
            .map(|addr| {
                Listen::new(Connection::Tcp(addr), Protocol::Grpc)
            }));

        let network = network::Configuration {
            peer_nodes,
            listen_to,
        };

        let config : config::Config = {
            let mut file = File::open(command_arguments.node_config.clone()).unwrap();
            match serde_yaml::from_reader(&mut file) {
                Err(e) => {
                    println!("config error: {}", e);
                    std::process::exit(1);
                },
                Ok(c) => c,
            }
        };

        let consensus = {
            if let Some(bft) = config.bft {
                Consensus::Bft(bft)
            } else if let Some(genesis) = config.genesis {
                Consensus::Genesis
            } else {
                println!("no consensus algorithm defined");
                std::process::exit(1);
            }
        };

        Settings {
            genesis_data_config: command_arguments.genesis_data_config.clone(),
            secret_config: command_arguments.secret.clone().or(config.secret_file).expect("secret config unspecified"),
            network: network,
            leadership: Leadership::from(!command_arguments.without_leadership.clone()),
            consensus: consensus,
            cmd_args: command_arguments,
        }
    }

    pub fn get_log_level(&self) -> log::LevelFilter {
        let log_level = match self.cmd_args.verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };
        log_level
    }

    /// read and parse the genesis data, from the file specified in the Settings
    pub fn read_genesis_data(&self) -> GenesisData {
        let filepath = &self.cmd_args.genesis_data_config;
        let mut f = File::open(filepath).unwrap();
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).unwrap();

        parse_genesis_data(&buffer[..])
    }
}
