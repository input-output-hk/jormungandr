use chain_addr::AddressReadable;
use chain_impl_mockchain::value::Value;
use std::net::SocketAddr;
use std::path::PathBuf;
use structopt::clap::{_clap_count_exprs, arg_enum};
use structopt::StructOpt;

use crate::blockcfg::genesis_data::{Discrimination, InitialUTxO, PublicKey};

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

    /// set to allow the creation of account without publishing certificate
    /// By default account are created only for the reward account.
    ///
    /// However it is possible to allows user of the blockchain to create an
    /// account too by just setting this parameter
    #[structopt(long = "allow-account-creation")]
    pub allow_account_creation: bool,

    /// set the address discrimination (`testing` or `production`).
    #[structopt(long = "discrimination")]
    pub address_discrimination: Discrimination,

    /// the linear fee constant parameter
    #[structopt(long = "linear-fee-constant", default_value = "0")]
    pub linear_fee_constant: u64,
    /// the linear fee coefficient parameter
    #[structopt(long = "linear-fee-coefficient", default_value = "0")]
    pub linear_fee_coefficient: u64,
    /// the linear fee certificate parameter
    #[structopt(long = "linear-fee-certificate", default_value = "0")]
    pub linear_fee_certificate: u64,
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

    /// initialize a new genesis configuration file
    #[structopt(name = "init")]
    Init(InitArguments),

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
