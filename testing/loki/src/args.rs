use jormungandr_testing_utils::testing::node::LogLevel;
use std::{net::SocketAddr, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab")]
pub struct Args {
    /// Path to the genesis block (the block0) of the blockchain
    #[structopt(long, short, parse(try_from_str))]
    pub genesis_block: PathBuf,

    /// Set the secret node config (in YAML format).
    #[structopt(long, short, parse(from_os_str))]
    pub secret: Option<PathBuf>,

    /// Specifies the address the node will listen.
    #[structopt(short = "a", long = "listen-address")]
    pub listen_address: Option<SocketAddr>,

    /// Log level
    #[structopt(long, short, default_value = "info")]
    pub log_level: LogLevel,
}
