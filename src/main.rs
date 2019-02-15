#![cfg_attr(feature = "with-bench", feature(test))]
extern crate bincode;
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
#[macro_use(o)]
extern crate slog;
extern crate rand;
extern crate slog_async;
extern crate slog_json;
extern crate slog_term;
extern crate structopt;

extern crate cardano;
extern crate cardano_storage;
extern crate chain_core;
extern crate chain_impl_mockchain;
extern crate chain_storage;
extern crate cbor_event;
extern crate exe_common;
extern crate network_core;
extern crate network_grpc;
extern crate protocol_tokio as protocol;

extern crate futures;
extern crate tokio;

extern crate cryptoxide;
extern crate curve25519_dalek;
extern crate generic_array;
extern crate sha2;

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

#[macro_use]
extern crate lazy_static;

#[macro_use]
pub mod log_wrapper;

pub mod blockcfg;
pub mod blockchain;
pub mod client;
pub mod clock;
pub mod consensus;
pub mod intercom;
pub mod leadership;
pub mod network;
pub mod secure;
pub mod settings;
pub mod state;
pub mod transaction;
pub mod utils;

use settings::Settings;
//use state::State;
use blockchain::{Blockchain, BlockchainR};
use futures::sync::mpsc::UnboundedSender;
use intercom::BlockMsg;
use intercom::NetworkBroadcastMsg;
use leadership::leadership_task;
use transaction::{transaction_task, TPool};
use utils::task::Tasks;

use std::sync::{mpsc::Receiver, Arc, RwLock};

use blockcfg::{genesis_data::GenesisData, mock::Mockchain as Cardano};
use chain_impl_mockchain::{
    key::PrivateKey,
    transaction::{SignedTransaction, TransactionId},
};

pub type TODO = u32;

fn node_private_key(ns: secure::NodeSecret) -> PrivateKey {
    let mut bytes = [0; 32];
    bytes.copy_from_slice(&ns.block_privatekey.as_ref()[0..32]);
    PrivateKey::normalize_bytes(bytes)
}

fn block_task(
    blockchain: BlockchainR<Cardano>,
    _clock: clock::Clock, // FIXME: use it or lose it
    r: Receiver<BlockMsg<Cardano>>,
    network_broadcast: UnboundedSender<NetworkBroadcastMsg<Cardano>>,
) {
    loop {
        let bquery = r.recv().unwrap();
        blockchain::process(&blockchain, bquery, &network_broadcast);
    }
}

fn startup_info(gd: &GenesisData, blockchain: &Blockchain<Cardano>, settings: &Settings) {
    println!(
        "k={} tip={}",
        gd.epoch_stability_depth,
        blockchain.get_tip()
    );
    println!("consensus: {:?}", settings.consensus);
}

// Expand the type with more variants
// when it becomes necessary to represent different error cases.
type Error = settings::Error;

fn run() -> Result<(), Error> {
    // # load parameters & config
    //
    // parse the command line arguments, the config files supplied
    // and setup the initial values
    let settings = Settings::load()?;

    settings.log_settings.apply();

    let genesis_data = settings.read_genesis_data().unwrap();

    let clock = {
        let initial_epoch = clock::ClockEpochConfiguration {
            slot_duration: genesis_data.slot_duration,
            slots_per_epoch: genesis_data.epoch_stability_depth * 10,
        };
        clock::Clock::new(genesis_data.start_time, initial_epoch)
    };

    let secret = secure::NodeSecret::load_from_file(settings.secret_config.as_path()).unwrap();

    //let mut state = State::new();

    let blockchain_data = Blockchain::new(
        genesis_data.clone(),
        secret.public.clone(),
        &settings.consensus,
    );

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

    network::bootstrap(&settings.network, blockchain.clone());

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

    let tpool_data: TPool<TransactionId, SignedTransaction> = TPool::new();
    let tpool = Arc::new(RwLock::new(tpool_data));

    // Validation of consensus settings should make sure that we always have
    // non-empty selection data.

    // initialize the transaction broadcast channel
    let (broadcast_sender, broadcast_receiver) = futures::sync::mpsc::unbounded();

    let transaction_task = {
        let tpool = tpool.clone();
        let blockchain = blockchain.clone();
        tasks.task_create_with_inputs("transaction", move |r| {
            transaction_task(blockchain, tpool, r)
        })
    };

    let block_task = {
        let blockchain = blockchain.clone();
        let clock = clock.clone();
        tasks.task_create_with_inputs("block", move |r| {
            block_task(blockchain, clock, r, broadcast_sender)
        })
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
            client_box: client_msgbox,
            transaction_box: transaction_msgbox,
            block_box: block_msgbox,
        };
        tasks.task_create("network", move || {
            network::run(config, broadcast_receiver, channels);
        });
    };

    if settings.leadership == settings::Leadership::Yes
    //    && leadership::selection::can_lead(&selection) == leadership::IsLeading::Yes
    {
        let tpool = tpool.clone();
        let clock = clock.clone();
        let block_task = block_task.clone();
        let blockchain = blockchain.clone();
        let pk = node_private_key(secret);
        tasks.task_create("leadership", move || {
            leadership_task(pk, tpool, blockchain, clock, block_task)
        });
    };

    // periodically cleanup (custom):
    //   storage cleanup/packing
    //   tpool.gc()

    // FIXME some sort of join so that the main thread does something ...
    tasks.join();

    Ok(())
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("jormungandr error: {}", e);
            std::process::exit(1);
        }
    }
}
