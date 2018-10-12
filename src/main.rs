#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
#[macro_use]
extern crate log;
extern crate env_logger;

extern crate cardano;
extern crate cardano_storage;
extern crate exe_common;

extern crate jormungandr;

use std::path::{PathBuf};

use jormungandr::gclock;

pub struct State {
}

impl State {
    pub fn new() -> Self {
        State {}
    }
}

fn main() {
    /// load parameters & config
    let mut state = State::new();

    /// bootstrap phase (peer discovery, download all the existing blocks)

    /// connect to peers
    /// core logic

    // setup_network
    // events:
    //  new connection:
    //    poll:
    //      recv_transaction:
    //         check_transaction_valid
    //         add transaction to pool
    //      recv_block:
    //         check block valid
    //         try to extend blockchain with block
    //         update utxo state
    //         flush transaction pool if any txid made it
    //      get block(s):
    //         try to answer
    //
    // periodically protocol speed:
    //   check elected
    //   if elected
    //     take set of transactions from pool
    //     create a block
    //     send it async to peers
    //
    // periodically cleanup:
    //   storage cleanup/packing

    println!("Hello, world!");
}
