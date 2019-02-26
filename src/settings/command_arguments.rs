use chain_addr::AddressReadable;
use chain_impl_mockchain::transaction::Value;
use std::net::SocketAddr;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::blockcfg::genesis_data::{InitialUTxO, PublicKey};

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

    /// Set the genesis data config (in JSON format) to use as configuration
    /// for the node's blockchain
    #[structopt(long = "genesis-config", parse(from_os_str))]
    pub genesis_data_config: PathBuf,
}

#[derive(StructOpt, Debug)]
pub struct InitArguments {
    /// set the address that will have all the initial funds associated to it.
    /// This is the wallet that will serve as faucet on testnets and as initial
    /// coin wallet for mainnets
    ///
    /// You will need to create an address (s) (make sure to save the
    /// spending key securely) and then you can add initial-utxos:
    ///
    /// ```text
    /// jormundandr --initial-utxos=ca1qvqsyqcyq5rqwzqfpg9scrgwpugpzysnzs23v9ccrydpk8qarc0jqxuzx4s@999999999
    /// ```
    #[structopt(long = "initial-utxos", parse(try_from_str = "parse_initial_utxo"))]
    pub initial_utxos: Vec<InitialUTxO>,

    /// set the time between the creation of 2 blocks. Value is a positive integer to
    /// be in seconds.
    #[structopt(
        short = "slot-duration",
        parse(try_from_str = "parse_duration"),
        default_value = "15"
    )]
    pub slot_duration: std::time::Duration,

    /// set the number of blocks that can be used to pack in the storage
    ///
    /// In the BFT paper it corresponds to the `t` parameter.
    #[structopt(long = "epoch-stability-depth", default_value = "10")]
    pub epoch_stability_depth: usize,

    /// one starting up the protocol will be in OBFT mode, you need to provide a list of
    /// authoritative public keys that will control the blockchain
    #[structopt(long = "bft-leader", parse(try_from_str))]
    pub bft_leaders: Vec<PublicKey>,
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

    /// initialize a new genesis configuration file
    #[structopt(name = "init")]
    Init(InitArguments),

    /// command to generate a new set of random key pair for the node to propose
    /// itself as a participating node or not
    #[structopt(name = "generate-keys")]
    GenerateKeys,
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

fn parse_duration(s: &str) -> Result<std::time::Duration, std::num::ParseIntError> {
    let time_seconds = s.parse::<u64>()?;
    Ok(std::time::Duration::new(time_seconds, 0))
}

fn parse_initial_utxo(s: &str) -> Result<InitialUTxO, String> {
    use std::str::FromStr;

    let value: Vec<_> = s.split("@").collect();
    if value.len() != 2 {
        return Err(format!("Expecting initial UTxO format: <address>@<value>"));
    }

    let address = match AddressReadable::from_str(&value[0]) {
        Err(error) => return Err(format!("Invalid initial UTxO's address: {}", error)),
        Ok(address) => address,
    };

    let value = match value[1].parse::<u64>() {
        Err(error) => return Err(format!("Invalid initial UTxO's value: {}", error)),
        Ok(v) => Value(v),
    };

    Ok(InitialUTxO {
        address: address,
        value: value,
    })
}
