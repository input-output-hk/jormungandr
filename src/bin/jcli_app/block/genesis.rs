extern crate chain_addr;
extern crate chain_core;
extern crate chain_impl_mockchain;
extern crate structopt;

use chain_core::property::{Block as _, Deserialize, Serialize};
use chain_impl_mockchain::block;
use jcli_app::utils::io;
use structopt::StructOpt;

mod yaml;

impl Genesis {
    pub fn exec(self) {
        match self {
            Genesis::Init => init_genesis_yaml(),
            Genesis::Encode(create_arguments) => encode_block_0(create_arguments),
            Genesis::Decode(info_arguments) => decode_block_0(info_arguments),
            Genesis::Hash(hash_arguments) => print_hash(hash_arguments),
        }
    }
}

fn init_genesis_yaml() {
    let path: Option<&'static str> = None;
    let out = io::open_file_write(&path).unwrap();

    yaml::documented_example(out, std::time::SystemTime::now()).unwrap()
}

fn encode_block_0(argument: Common) {
    // read yaml file
    let yaml: yaml::Genesis =
        serde_yaml::from_reader(io::open_file_read(&argument.input_file).unwrap()).unwrap();

    let block = yaml.to_block();

    block
        .serialize(io::open_file_write(&argument.output_file).unwrap())
        .unwrap()
}

fn decode_block_0(argument: Common) {
    let block = open_block(&argument.input_file);
    let yaml = yaml::Genesis::from_block(&block);

    serde_yaml::to_writer(io::open_file_write(&argument.output_file).unwrap(), &yaml).unwrap();
}

fn print_hash(argument: Input) {
    let block = open_block(&argument.input_file);
    println!("{}", block.id());
}

/// create block 0 of the blockchain (i.e. the genesis block)
#[derive(StructOpt)]
#[structopt(name = "genesis", rename_all = "kebab-case")]
pub enum Genesis {
    /// Create a default Genesis file with appropriate documentation
    /// to help creating the YAML file
    Init,

    /// create the block 0 file (the genesis block of the blockchain)
    /// from a given yaml file
    ///
    Encode(Common),

    /// Decode the block 0 and print the corresponding YAML file
    Decode(Common),

    /// print the block hash (aka the block id) of the block 0
    Hash(Input),
}

#[derive(StructOpt)]
pub struct Input {
    /// the file path to the genesis file defining the block 0
    ///
    /// If not available the command will expect to read the configuration from
    /// the standard input.
    #[structopt(long = "input", parse(from_os_str), name = "FILE_INPUT")]
    input_file: Option<std::path::PathBuf>,
}

#[derive(StructOpt)]
pub struct Common {
    /// the file path to the genesis file defining the block 0
    ///
    /// If not available the command will expect to read the configuration from
    /// the standard input.
    #[structopt(long = "input", parse(from_os_str), name = "FILE_INPUT")]
    input_file: Option<std::path::PathBuf>,

    /// the file path to the block to create
    ///
    /// If not available the command will expect to write the block to
    /// to the standard output
    #[structopt(long = "output", parse(from_os_str), name = "FILE_OUTPUT")]
    output_file: Option<std::path::PathBuf>,
}

fn open_block<P: AsRef<std::path::Path>>(path: &Option<P>) -> block::Block {
    let reader = io::open_file_read(path).unwrap();
    block::Block::deserialize(reader).unwrap()
}
