#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate structopt;

extern crate cardano;
extern crate cardano_storage;
extern crate exe_common;
extern crate protocol_tokio as protocol;
extern crate futures;
extern crate tokio;

pub mod clock;
pub mod blockchain;
pub mod transaction;
pub mod state;
pub mod network;
pub mod utils;
pub mod intercom;
pub mod settings;
pub mod blockcfg;

use std::path::{PathBuf};

use settings::Settings;
//use state::State;
use transaction::{TPool};
use blockchain::{Blockchain, BlockchainR};
use utils::task::{Tasks, Task, TaskMessageBox};
use intercom::{BlockMsg, ClientMsg, TransactionMsg};

use blockcfg::*;

use std::sync::{Arc, RwLock, mpsc::Receiver};
use std::{time, thread};

use cardano_storage::StorageConfig;

pub type TODO = u32;
pub type TPoolR = Arc<RwLock<TPool<TransactionId, Transaction>>>;

fn transaction_task(_tpool: TPoolR, r: Receiver<TransactionMsg>) {
    loop {
        let tquery = r.recv().unwrap();
        println!("transaction received: {}", tquery)
    }
}

fn block_task(blockchain: BlockchainR, clock: clock::Clock, r: Receiver<BlockMsg>) {
    loop {
        let bquery = r.recv().unwrap();
        blockchain::process(&blockchain, bquery);
    }
}

fn client_task(_blockchain: BlockchainR, r: Receiver<ClientMsg>) {
    loop {
        let query = r.recv().unwrap();
        println!("client query received: {:?}", query)
    }
}

fn leadership_task(tpool: TPoolR, blockchain: BlockchainR, clock: clock::Clock) {
    loop {
        let d = clock.wait_next_slot();
        let (epoch, idx, next_time) = clock.current_slot().unwrap();
        println!("slept for {:?} epoch {} slot {} next_slot {:?}", d, epoch.0, idx, next_time);
        let len = {
            let t = tpool.read().unwrap();
            (*t).content.len()
        };

        // TODO: check if this node is "elected" (by design or by stake) for this slot
        let elected = true;

        if elected {
            // create a new block to broadcast:
            // * get the transactions to put in the transactions
            // * mint the block
            // * sign it
            let latest_tip = {
                let b = blockchain.read().unwrap();
                b.get_tip()
            };

            println!("leadership create tpool={} transactions tip={}", len, latest_tip);

            // SIMULATING busy task: take between 1 to 21 seconds.
            {
                let v = 1u64 + (rand::random::<u64>() % 20);
                thread::sleep(time::Duration::from_secs(v))
            };
            // TODO: send it to block thread for appending/broadcasting
        }

    }
}

fn startup_info(gd: &GenesisData) {
    println!("protocol magic={} prev={} k={}", gd.protocol_magic, gd.genesis_prev, gd.epoch_stability_depth);
}

fn main() {
    // # load parameters & config
    //
    // parse the command line arguments, the config files supplied
    // and setup the initial values
    let settings = Settings::load();

    env_logger::Builder::from_default_env()
        .filter_level(settings.get_log_level())
        .init();

    let genesis_data = settings.read_genesis_data();

    startup_info(&genesis_data);

    let clock = {
        let initial_epoch = clock::ClockEpochConfiguration {
            slot_duration: genesis_data.slot_duration,
            slots_per_epoch: genesis_data.epoch_stability_depth * 10,
        };
        clock::Clock::new(genesis_data.start_time, initial_epoch)
    };

    //let mut state = State::new();

    let pathbuf = PathBuf::from(r"pool-storage"); // FIXME HARDCODED should come from config
    let storage_config = StorageConfig::new(&pathbuf);
    let blockchain_data = Blockchain::from_storage(&genesis_data, &storage_config);
    let blockchain = Arc::new(RwLock::new(blockchain_data));

    let mut tasks = Tasks::new();

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

    // Read block state
    // init storage
    // create blockchain storage

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

    let tpool_data : TPool<TransactionId, Transaction> = TPool::new();
    let tpool = Arc::new(RwLock::new(tpool_data));

    let transaction_task = {
        let tpool = Arc::clone(&tpool);
        tasks.task_create_with_inputs("transaction", move |r| transaction_task(tpool, r))
    };

    let block_task = {
        let blockchain = Arc::clone(&blockchain);
        let clock = clock.clone();
        tasks.task_create_with_inputs("block", move |r| block_task(blockchain, clock, r))
    };

    let client_task = {
        let blockchain = Arc::clone(&blockchain);
        tasks.task_create_with_inputs("client-query", move |r| client_task(blockchain, r))
    };

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
    {
        let client_msgbox = client_task.clone();
        let transaction_msgbox = transaction_task.clone();
        let block_msgbox = block_task.clone();
        let config = settings.network.clone();
        let channels = network::Channels {
            client_box:      client_msgbox,
            transaction_box: transaction_msgbox,
            block_box:       block_msgbox,
        };
        tasks.task_create("network", move || {
            network::run(config, channels);
        });
    };

    {
        let tpool = Arc::clone(&tpool);
        let clock = clock.clone();
        let blockchain = Arc::clone(&blockchain);
        tasks.task_create("leadership", move || leadership_task(tpool, blockchain, clock));
    };

    // periodically cleanup (custom):
    //   storage cleanup/packing
    //   tpool.gc()

    // FIXME some sort of join so that the main thread does something ...
    tasks.join();
}
