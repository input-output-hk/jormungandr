use crate::blockcfg::BlockConfig;
use crate::blockchain::BlockchainR;
use crate::settings::network::{Connection, Peer};

use network_core::client::block::BlockService;
use network_grpc::client::Client;

use tokio::prelude::*;
use tokio::{io, runtime::current_thread};

pub fn bootstrap_from_target<P, B>(peer: P, blockchain: BlockchainR<B>)
where
    B: BlockConfig,
    P: tokio_connect::Connect<Error = io::Error> + 'static,
{
    let bootstrap = Client::connect(peer)
        .map_err(|e| {
            error!("failed to connect to bootstrap peer: {:?}", e);
        })
        .and_then(|mut client| {
            let tip = blockchain.read().unwrap().tip();
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
{
    stream
        .fold(blockchain, |blockchain, block| {
            debug!("received block from the bootstrap node: {:#?}", &block);
            blockchain.write().unwrap().handle_incoming_block(block);
            future::ok(blockchain)
        })
        .map(|_| ())
        .map_err(|e| {
            error!("bootstrap block streaming failed: {:?}", e);
        })
}
