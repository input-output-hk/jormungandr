extern crate chain_core;
extern crate chain_impl_mockchain;
extern crate structopt;

use chain_core::property::{Deserialize, Serialize};
use chain_impl_mockchain::block::{Block, BlockBuilder};
use structopt::StructOpt;

fn main() {
    match Command::from_args() {
        Command::Create(create_arguments) => create_block(create_arguments),
        Command::Add(add_arguments) => add_to_block(add_arguments),
        Command::Info(info_arguments) => print_block(info_arguments),
    }
}

fn create_block(argument: Common) {
    if argument.block_exists() {
        panic!("Block already exists")
    }

    let block = BlockBuilder::new().make_genesis_block();
    let file = argument.open_block_file_write();
    block.serialize(file).unwrap();
}

fn print_block(argument: Common) {
    let block = argument.open_block();
    println!("{:#?}", block);
}

fn add_to_block(argument: AddArgs) {
    let block = argument.common.open_block();
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
}

#[derive(StructOpt)]
struct AddArgs {
    #[structopt(flatten)]
    common: Common,
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

    fn open_block_file_write(&self) -> Box<dyn std::io::Write> {
        if let Some(path) = &self.block {
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

    fn open_block_file_read(&self) -> Box<dyn std::io::BufRead> {
        if let Some(path) = &self.block {
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

    fn open_block(&self) -> Block {
        let reader = self.open_block_file_read();
        Block::deserialize(reader).unwrap()
    }
}
