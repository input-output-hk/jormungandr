use crate::blockcfg::BlockConfig;
use crate::blockchain::BlockchainR;

use chain_core::property::{self, Deserialize};
use network_core::client::block::BlockService;
use network_grpc::client::Client;

use tokio::prelude::*;
use tokio::{executor::DefaultExecutor, io, runtime::current_thread};

use std::fmt::Debug;

pub fn bootstrap_from_target<P, B>(peer: P, blockchain: BlockchainR<B>)
where
    B: BlockConfig,
    P: tokio_connect::Connect<Error = io::Error> + 'static,
    P::Connected: Send,
    <B::Block as Deserialize>::Error: Send + Sync,
    <B::BlockHash as Deserialize>::Error: Send + Sync,
    <B::Ledger as property::Ledger>::Update: Clone,
    <B::Settings as property::Settings>::Update: Clone,
    <B::Leader as property::LeaderSelection>::Update: Clone,
{
    let bootstrap = Client::connect(peer, DefaultExecutor::current())
        .map_err(|e| {
            error!("failed to connect to bootstrap peer: {:?}", e);
        })
        .and_then(|mut client| {
            let tip = blockchain.read().unwrap().get_tip();
            client
                .pull_blocks_to_tip(&[tip])
                .map_err(|e| {
                    error!("PullBlocksToTip request failed: {:?}", e);
                })
                .and_then(|stream| bootstrap_from_stream(blockchain, stream))
        });

    match current_thread::block_on_all(bootstrap) {
        Ok(()) => debug!("bootstrap complete"),
        Err(()) => {
            // All specific errors should be logged and mapped to () in
            // future/stream error handling combinators.
        }
    }
}

fn bootstrap_from_stream<B, S>(
    blockchain: BlockchainR<B>,
    stream: S,
) -> impl Future<Item = (), Error = ()>
where
    B: BlockConfig,
    S: Stream<Item = <B as BlockConfig>::Block>,
    S::Error: Debug,
    <B::Ledger as property::Ledger>::Update: Clone,
    <B::Settings as property::Settings>::Update: Clone,
    <B::Leader as property::LeaderSelection>::Update: Clone,
{
    stream
        .fold(blockchain, |blockchain, block| {
            // debug!("received block from the bootstrap node: {:#?}", &block);
            blockchain.write().unwrap().handle_incoming_block(block);
            future::ok(blockchain)
        })
        .map(|_| ())
        .map_err(|e| {
            error!("bootstrap block streaming failed: {:?}", e);
        })
}
