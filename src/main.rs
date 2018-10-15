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
use jormungandr::tpool::TPool;

use cardano::tx::{TxId, TxAux};

pub struct State {
}

impl State {
    pub fn new() -> Self {
        State {}
    }
}

fn main() {
    // # load parameters & config
    //
    // parse the command line arguments, the config files supplied
    // and setup the initial values
    let mut state = State::new();

    // # Bootstrap phase
    //
    // done at every startup: we need to bootstrap from whatever local state (including nothing)
    // to the latest network state (or close to latest). until this happen, we don't participate in the network
    // (no block creation) and our network connection(s) is only use to download data.
    //
    // Various aspects to do, similar to hermes:
    // * download all the existing blocks
    // * verify all the downloaded blocks
    // * network / peer discoveries (?)
    // * gclock sync ?

    // ** TODO **

    // # Active phase
    //
    // now that we have caught up (or almost caught up) we download blocks from neighbor nodes,
    // listen to announcements and actively listen to synchronous queries
    //
    // There's two simultaenous roles to this:
    // * Leader: decided after global or local evaluation. Need to create and propagate a block
    // * Non-Leader: always. receive (pushed-) blocks from other peers, investigate the correct blockchain updates
    //
    // Also receive synchronous connection queries:
    // * new nodes subscribing to updates (blocks, transactions)
    // * client GetBlocks/Headers ...

    let mut tpool : TPool<TxId, TxAux> = TPool::new();

    // ** TODO **
    // setup_network
    //  connection-events:
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
    // periodically (protocol speed):
    //   check elected
    //   if elected
    //     take set of transactions from pool
    //     create a block
    //     send it async to peers
    //
    // periodically cleanup (custom):
    //   storage cleanup/packing
    //   tpool.gc()

    println!("Hello, world!");
}
