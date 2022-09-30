use crate::jcli_lib::utils::io;
use chain_core::{
    packer::Codec,
    property::{Block as _, Deserialize, ReadError, Serialize, WriteError},
};
use chain_impl_mockchain::{
    block::Block,
    ledger::{self, Ledger},
};
use jormungandr_lib::interfaces::{
    block0_configuration_documented_example, Block0Configuration, Block0ConfigurationError,
};
use std::{
    io::{BufRead, Write},
    path::PathBuf,
};
use structopt::StructOpt;
use thiserror::Error;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid input file path '{path}'")]
    InputInvalid {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("invalid output file path '{path}'")]
    OutputInvalid {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("block file corrupted")]
    BlockFileCorrupted(#[source] ReadError),
    #[error("genesis file corrupted")]
    GenesisFileCorrupted(#[source] serde_yaml::Error),
    #[error("generated block is not a valid genesis block")]
    GeneratedBlock0Invalid(#[from] ledger::Error),
    #[error("failed to serialize block")]
    BlockSerializationFailed(#[source] WriteError),
    #[error("failed to serialize genesis")]
    GenesisSerializationFailed(#[source] serde_yaml::Error),
    #[error("failed to build genesis from block 0")]
    BuildingGenesisFromBlock0Failed(#[from] Block0ConfigurationError),
}

impl Genesis {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Genesis::Init => {
                init_genesis_yaml();
                Ok(())
            }
            Genesis::Encode(create_arguments) => encode_block_0(create_arguments),
            Genesis::Decode(info_arguments) => decode_block_0(info_arguments),
            Genesis::Hash(hash_arguments) => print_hash(hash_arguments),
        }
    }
}

fn init_genesis_yaml() {
    println!("{}", block0_configuration_documented_example());
}

fn encode_block_0(common: Common) -> Result<(), Error> {
    let reader = common.input.open()?;
    let genesis: Block0Configuration =
        serde_yaml::from_reader(reader).map_err(Error::GenesisFileCorrupted)?;
    let block = genesis.to_block();
    Ledger::new(block.id(), block.fragments())?;
    block
        .serialize(&mut Codec::new(common.open_output()?))
        .map_err(Error::BlockSerializationFailed)
}

fn decode_block_0(common: Common) -> Result<(), Error> {
    let block = common.input.load_block()?;
    let yaml = Block0Configuration::from_block(&block)?;
    serde_yaml::to_writer(common.open_output()?, &yaml).map_err(Error::GenesisSerializationFailed)
}

fn print_hash(input: Input) -> Result<(), Error> {
    let block = input.load_block()?;
    println!("{}", block.id());
    Ok(())
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
    pub input_file: Option<std::path::PathBuf>,
}

impl Input {
    pub fn open(&self) -> Result<impl BufRead, Error> {
        open_block_file(&self.input_file)
    }

    pub fn load_block(&self) -> Result<Block, Error> {
        let reader = self.open()?;
        load_block(reader)
    }
}

pub fn open_block_file(input_file: &Option<PathBuf>) -> Result<impl BufRead, Error> {
    io::open_file_read(input_file).map_err(|source| Error::InputInvalid {
        source,
        path: input_file.clone().unwrap_or_default(),
    })
}

pub fn load_block(block_reader: impl BufRead) -> Result<Block, Error> {
    Block::deserialize(&mut Codec::new(block_reader)).map_err(Error::BlockFileCorrupted)
}

#[derive(StructOpt)]
pub struct Common {
    #[structopt(flatten)]
    pub input: Input,

    /// the file path to the block to create
    ///
    /// If not available the command will expect to write the block to
    /// to the standard output
    #[structopt(long = "output", parse(from_os_str), name = "FILE_OUTPUT")]
    pub output_file: Option<std::path::PathBuf>,
}

impl Common {
    pub fn open_output(&self) -> Result<impl Write, Error> {
        open_output(&self.output_file)
    }
}

pub fn open_output(path: &Option<PathBuf>) -> Result<impl Write, Error> {
    io::open_file_write(path).map_err(|source| Error::OutputInvalid {
        source,
        path: path.clone().unwrap_or_default(),
    })
}
