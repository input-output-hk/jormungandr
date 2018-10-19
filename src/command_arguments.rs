use std::net::SocketAddr;

pub use structopt::{StructOpt};

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
}
