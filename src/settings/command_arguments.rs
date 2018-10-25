use std::net::SocketAddr;
use std::path::PathBuf;

use structopt::{StructOpt};

#[derive(StructOpt, Debug)]
#[structopt(
        name = "jormungandr",
        raw(setting = "structopt::clap::AppSettings::ColoredHelp"),
    )
]
pub struct CommandArguments {
    /// activate the verbosity, the more occurrences the more verbose.
    /// (-v, -vv, -vvv)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    pub verbose: u8,

    /// the address to listen inbound connections from. The network will
    /// open an listening socket to the given address. You might need to have
    /// special privileges to open the TcpSocket from this address.
    #[structopt(long = "listen-from", parse(try_from_str))]
    pub listen_addr: SocketAddr,

    /// list of the nodes to connect too. They are the nodes we know
    /// we need to connect too and to start processing blocks, transactions
    /// and participate with.
    ///
    #[structopt(long = "connect-to", parse(try_from_str))]
    pub connect_to: Vec<SocketAddr>,

    /// Set the genesis data config (in JSON format) to use as configuration
    /// for the node's blockchain
    #[structopt(long = "genesis-config", parse(from_os_str))]
    pub genesis_data_config: PathBuf,
}

impl CommandArguments {
    /// load the command arguments from the command line args
    ///
    /// on error during reading the command line arguments, the
    /// function will print an error message and will terminate
    /// the process.
    ///
    pub fn load() -> Self { Self::from_args() }
}
