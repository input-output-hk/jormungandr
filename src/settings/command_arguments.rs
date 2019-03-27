use std::net::SocketAddr;
use std::path::PathBuf;
use structopt::clap::{_clap_count_exprs, arg_enum};
use structopt::StructOpt;

use crate::settings::logging::LogFormat;

#[derive(StructOpt, Debug)]
pub struct StartArguments {
    /// The address to listen for inbound legacy protocol connections at.
    /// The program will open a listening socket on the given address.
    /// You might need to have special privileges to open the TCP socket
    /// at this address.
    #[structopt(long = "legacy-listen", parse(try_from_str))]
    pub ntt_listen: Vec<SocketAddr>,

    /// The address to listen for inbound gRPC connections at.
    /// The program will open a listening socket on the given address.
    /// You might need to have special privileges to open the TCP socket
    /// at this address.
    #[structopt(long = "grpc-listen", parse(try_from_str))]
    pub grpc_listen: Vec<SocketAddr>,

    /// List of the nodes to connect to using the legacy protocol.
    /// These are the nodes we know we need to connect to and
    /// start processing blocks, transactions and participate with.
    ///
    #[structopt(long = "legacy-connect", parse(try_from_str))]
    pub ntt_connect: Vec<SocketAddr>,

    /// List of the nodes to connect to using the grpc protocol.
    /// These are the nodes we know we need to connect to and
    /// start processing blocks, transactions and participate with.
    ///
    #[structopt(long = "grpc-connect", parse(try_from_str))]
    pub grpc_connect: Vec<SocketAddr>,

    /// Work without the leadership task.
    #[structopt(long = "without-leadership")]
    pub without_leadership: bool,

    /// Path to the blockchain pool storage directory
    #[structopt(long = "storage", parse(from_os_str))]
    pub storage: Option<PathBuf>,

    /// Set the node config (in YAML format) to use as general configuration
    #[structopt(long = "config", parse(from_os_str))]
    pub node_config: PathBuf,

    /// Set the secret node config (in YAML format)
    #[structopt(long = "secret", parse(from_os_str))]
    pub secret: Option<PathBuf>,

    /// Set the block 0 (the genesis block) of the blockchain
    #[structopt(long = "genesis-block", parse(from_os_str))]
    pub block_0: PathBuf,
}

#[derive(StructOpt, Debug)]
pub struct GeneratePrivKeyArguments {
    /// Type of a private key
    ///
    /// value values are: ed25519, ed25510bip32, ed25519extended, curve25519_2hashdh
    #[structopt(long = "type")]
    pub key_type: GenPrivKeyType,
}

#[derive(StructOpt, Debug)]
pub struct GeneratePubKeyArguments {
    /// the source private key to extract the public key from
    ///
    /// if no value passed, the private key will be read from the
    /// standard input
    #[structopt(name = "PRIVATE_KEY")]
    pub private_key: Option<String>,
}

arg_enum! {
    #[derive(StructOpt, Debug)]
    pub enum GenPrivKeyType {
        Ed25519,
        Ed25519Bip32,
        Ed25519Extended,
        FakeMMM,
        Curve25519_2HashDH,
    }
}

#[derive(StructOpt, Debug)]
#[structopt(
    name = "jormungandr",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
pub struct CommandLine {
    /// activate the verbosity, the more occurrences the more verbose.
    /// (-v, -vv, -vvv)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    pub verbose: u8,

    /// Set format of the log emitted. Can be "json" or "plain"
    #[structopt(long = "log-format", parse(try_from_str), default_value = "plain")]
    pub log_format: LogFormat,

    #[structopt(subcommand)]
    pub command: Command,
}

#[derive(StructOpt, Debug)]
pub enum Command {
    /// start jormungandr service and start participating to the network
    #[structopt(name = "start")]
    Start(StartArguments),

    /// generate a random private key and print it to stdout encoded in bech32
    #[structopt(name = "generate-priv-key")]
    GeneratePrivKey(GeneratePrivKeyArguments),

    /// generates a public key corresponding to a private key,
    /// reads private from stdin and prints its public to stdout, both encoded in bech32
    #[structopt(name = "generate-pub-key")]
    GeneratePubKey(GeneratePubKeyArguments),
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
