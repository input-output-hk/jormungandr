extern crate chain_addr;
extern crate chain_core;
extern crate chain_impl_mockchain;
extern crate structopt;

use chain_addr::Address;
use chain_core::property::{Block as _, Deserialize, Serialize};
use chain_impl_mockchain::{
    block::{Block, BlockBuilder, Message},
    transaction::SignedTransaction,
};
use std::path::Path;
use structopt::StructOpt;

fn main() {
    match Command::from_args() {
        Command::Create(create_arguments) => create_block(create_arguments),
        Command::Add(add_arguments) => add_to_block(add_arguments),
        Command::Info(info_arguments) => print_block(info_arguments),
        Command::Hash(hash_arguments) => print_hash(hash_arguments),
    }
}

fn create_block(argument: Common) {
    if argument.block_exists() {
        panic!("Block already exists")
    }

    let block = BlockBuilder::new().make_genesis_block();
    let file = open_file_write(&argument.block);
    block.serialize(file).unwrap();
}

fn print_block(argument: Common) {
    let block = argument.open_block();
    println!("{:#?}", block);
}

fn print_hash(argument: Common) {
    let block = argument.open_block();
    println!("{}", block.id());
}

fn add_to_block(argument: AddArgs) {
    let mut builder: BlockBuilder = argument.common.open_block().into();

    if argument.transaction.is_some() {
        builder.transaction(argument.open_transaction());
    }

    let block = builder.make_genesis_block();
    let file = open_file_write(&argument.common.block);
    block.serialize(file).unwrap();
    println!("{:#?}", block);
}

/// Jormungandr block tooling and helper
///
/// Command line to create or display the information of a given block.
#[derive(StructOpt)]
#[structopt(
    name = "genesis",
    rename_all = "kebab-case",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
enum Command {
    /// create a new block
    Create(Common),

    /// add entries to the block
    Add(AddArgs),

    /// display the content of a block in human readable format
    Info(Common),

    /// print the block hash (aka the block id)
    Hash(Common),
}

#[derive(StructOpt)]
struct AddArgs {
    #[structopt(flatten)]
    common: Common,

    /// message to add in the block
    transaction: Option<std::path::PathBuf>,
}

#[derive(StructOpt)]
struct Common {
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

    fn open_block(&self) -> Block {
        let reader = open_file_read(&self.block);
        Block::deserialize(reader).unwrap()
    }
}

impl AddArgs {
    fn open_transaction(&self) -> SignedTransaction<Address> {
        let reader = open_file_read(&self.transaction);
        SignedTransaction::deserialize(reader).unwrap()
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
