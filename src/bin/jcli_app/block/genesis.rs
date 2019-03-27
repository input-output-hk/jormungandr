extern crate chain_addr;
extern crate chain_core;
extern crate chain_impl_mockchain;
extern crate structopt;

use chain_core::property::{Block as _, Deserialize, Serialize};
use chain_impl_mockchain::block::{self, BlockBuilder};
use jcli_app::utils::io;
use structopt::StructOpt;

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
    unimplemented!()
}

fn encode_block_0(argument: Common) {
    if argument.block_exists() {
        panic!("Block already exists")
    }

    let block = BlockBuilder::new().make_genesis_block();
    let file = io::open_file_write(&argument.block);
    block.serialize(file).unwrap();
}

fn decode_block_0(argument: Common) {
    let block = argument.open_block();
    println!("{:#?}", block);
}

fn print_hash(argument: Common) {
    let block = argument.open_block();
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
    Hash(Common),
}

#[derive(StructOpt)]
pub struct Common {
    /// the file path to the block to create/update/display
    ///
    /// If not available the command will expect to read the block from
    /// the standard input and/or write the result to the standard output
    #[structopt(parse(from_os_str), name = "FILE")]
    block: Option<std::path::PathBuf>,
}

impl Common {
    fn block_exists(&self) -> bool {
        if let Some(path) = &self.block {
            path.is_file()
        } else {
            false
        }
    }

    fn open_block(&self) -> block::Block {
        let reader = io::open_file_read(&self.block);
        block::Block::deserialize(reader).unwrap()
    }
}
