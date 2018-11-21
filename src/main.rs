extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
#[macro_use]
extern crate log;
extern crate rand;
extern crate env_logger;
#[macro_use]
extern crate structopt;

extern crate cardano;
extern crate cardano_storage;
#[macro_use]
extern crate cbor_event;
extern crate exe_common;
extern crate protocol_tokio as protocol;
extern crate futures;
extern crate tokio;

pub mod clock;
pub mod blockchain;
pub mod transaction;
pub mod state;
pub mod leadership;
pub mod network;
pub mod utils;
pub mod intercom;
pub mod settings;
pub mod blockcfg;
pub mod client;
pub mod secure;

use std::path::{PathBuf};

use settings::Settings;
//use state::State;
use transaction::{TPool};
use blockchain::{Blockchain, BlockchainR};
use utils::task::{Tasks};
use intercom::{BlockMsg, TransactionMsg};
use leadership::{leadership_task, Selection};
use futures::sync::mpsc::UnboundedSender;
use intercom::NetworkBroadcastMsg;

use blockcfg::*;

use std::sync::{Arc, RwLock, mpsc::Receiver};

use cardano_storage::{StorageConfig};

pub type TODO = u32;
pub type TPoolR = Arc<RwLock<TPool<TransactionId, Transaction>>>;

fn transaction_task(_tpool: TPoolR, r: Receiver<TransactionMsg>) {
    loop {
        let tquery = r.recv().unwrap();
        println!("transaction received: {}", tquery)
    }
}

fn block_task(blockchain: BlockchainR, selection: Arc<Selection>, clock: clock::Clock, r: Receiver<BlockMsg>, network_broadcast: UnboundedSender<NetworkBroadcastMsg>) {
    loop {
        let bquery = r.recv().unwrap();
        blockchain::process(&blockchain, &selection, bquery, &network_broadcast);
    }
}

fn startup_info(gd: &GenesisData, blockchain: &Blockchain, settings: &Settings) {
    let (tip, tip_date) = blockchain.get_tip();
    println!("protocol magic={} prev={} k={} tip={} ({})", gd.protocol_magic, gd.genesis_prev, gd.epoch_stability_depth, tip, tip_date);
    println!("consensus: {:?}", settings.consensus);
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

    let clock = {
        let initial_epoch = clock::ClockEpochConfiguration {
            slot_duration: genesis_data.slot_duration,
            slots_per_epoch: genesis_data.epoch_stability_depth * 10,
        };
        clock::Clock::new(genesis_data.start_time, initial_epoch)
    };

    let secret = secure::NodeSecret::load_from_file(settings.secret_config.as_path()).unwrap();

    //let mut state = State::new();

    let pathbuf = PathBuf::from(r"pool-storage"); // FIXME HARDCODED should come from config
    let storage_config = StorageConfig::new(&pathbuf);
    let blockchain_data = Blockchain::from_storage(genesis_data.clone(), &storage_config);

    startup_info(&genesis_data, &blockchain_data, &settings);

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

    let selection_data = leadership::selection::prepare(&secret.to_public(), &settings.consensus).unwrap();
    let selection = Arc::new(selection_data);

    // initialize the transaction broadcast channel
    let (broadcast_sender, broadcast_receiver) = futures::sync::mpsc::unbounded();

    let transaction_task = {
        let tpool = tpool.clone();
        tasks.task_create_with_inputs("transaction", move |r| transaction_task(tpool, r))
    };

    let block_task = {
        let blockchain = blockchain.clone();
        let clock = clock.clone();
        let selection = Arc::clone(&selection);
        tasks.task_create_with_inputs("block", move |r| block_task(blockchain, selection, clock, r, broadcast_sender))
    };

    let client_task = {
        let blockchain = blockchain.clone();
        tasks.task_create_with_inputs("client-query", move |r| client::client_task(blockchain, r))
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
            network::run(config, broadcast_receiver, channels);
        });
    };

    if settings.leadership == settings::Leadership::Yes && leadership::selection::can_lead(&selection) == leadership::IsLeading::Yes {
        let tpool = tpool.clone();
        let clock = clock.clone();
        let selection = Arc::clone(&selection);
        let block_task = block_task.clone();
        let blockchain = blockchain.clone();
        tasks.task_create("leadership", move || leadership_task(secret, selection, tpool, blockchain, clock, block_task));
    };

    // periodically cleanup (custom):
    //   storage cleanup/packing
    //   tpool.gc()

    // FIXME some sort of join so that the main thread does something ...
    tasks.join();
}
