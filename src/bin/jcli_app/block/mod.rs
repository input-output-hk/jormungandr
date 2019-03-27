extern crate chain_addr;
extern crate chain_core;
extern crate chain_impl_mockchain;
extern crate structopt;

use chain_core::property::{Block as _, Deserialize, Serialize};
use chain_impl_mockchain::block::{self, BlockBuilder};
use std::path::Path;
use structopt::StructOpt;

impl Genesis {
    pub fn exec(self) {
        match self {
            Genesis::Encode(create_arguments) => encode_block_0(create_arguments),
            Genesis::Decode(info_arguments) => decode_block_0(info_arguments),
            Genesis::Hash(hash_arguments) => print_hash(hash_arguments),
        }
    }
}

fn encode_block_0(argument: Common) {
    if argument.block_exists() {
        panic!("Block already exists")
    }

    let block = BlockBuilder::new().make_genesis_block();
    let file = open_file_write(&argument.block);
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
    /// create a new block
    Encode(Common),

    /// display the content of a block in human readable format
    Decode(Common),

    /// print the block hash (aka the block id)
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
        let reader = open_file_read(&self.block);
        block::Block::deserialize(reader).unwrap()
    }
}

/// open the given file path as a writable stream, or stdout if no path
/// provided
fn open_file_write<P: AsRef<Path>>(path: &Option<P>) -> Box<dyn std::io::Write> {
    if let Some(path) = path {
        Box::new(
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(false)
                .append(false)
                .open(path)
                .unwrap(),
        )
    } else {
        Box::new(std::io::stdout())
    }
}

/// open the given file path as a readable stream, or stdin if no path
/// provided
fn open_file_read<P: AsRef<Path>>(path: &Option<P>) -> Box<dyn std::io::BufRead> {
    if let Some(path) = path {
        Box::new(std::io::BufReader::new(
            std::fs::OpenOptions::new()
                .create(false)
                .write(false)
                .read(true)
                .append(false)
                .open(path)
                .unwrap(),
        ))
    } else {
        Box::new(std::io::BufReader::new(std::io::stdin()))
    }
}
