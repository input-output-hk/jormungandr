use super::super::BlkCfg;
use crate::blockchain::BlockchainR;
use blockcfg::Block;

use chain_core::property::{Deserialize, HasHeader};
use network_core::client::block::BlockService as _;
use network_grpc::client::Client;

use tokio::prelude::*;
use tokio::{executor::DefaultExecutor, runtime::current_thread};

use http;
use std::fmt::Debug;

pub fn bootstrap_from_target<P>(peer: P, blockchain: BlockchainR, origin: http::Uri)
where
    P: tower_service::Service<(), Error = std::io::Error> + 'static,
    <P as tower_service::Service<()>>::Response:
        tokio::io::AsyncWrite + tokio::io::AsyncRead + 'static + Send,
    <Block as Deserialize>::Error: Send + Sync,
{
    let bootstrap = Client::connect(peer, DefaultExecutor::current(), origin)
        .map_err(|e| {
            error!("failed to connect to bootstrap peer: {:?}", e);
        })
        .and_then(
            |mut client: Client<
                BlkCfg,
                <P as tower_service::Service<()>>::Response,
                DefaultExecutor,
            >| {
                let tip = blockchain.read().unwrap().get_tip();
                client
                    .pull_blocks_to_tip(&[tip])
                    .map_err(|e| {
                        error!("PullBlocksToTip request failed: {:?}", e);
                    })
                    .and_then(|stream| bootstrap_from_stream(blockchain, stream))
            },
        );

    match current_thread::block_on_all(bootstrap) {
        Ok(()) => debug!("bootstrap complete"),
        Err(()) => {
            // All specific errors should be logged and mapped to () in
            // future/stream error handling combinators.
        }
    }
}

fn bootstrap_from_stream<S>(
    blockchain: BlockchainR,
    stream: S,
) -> impl Future<Item = (), Error = ()>
where
    S: Stream<Item = Block>,
    S::Error: Debug,
{
    stream
        .fold(blockchain, |blockchain, block| {
            debug!(
                "received block from the bootstrap node: {:#?}",
                block.header()
            );
            let res = blockchain.write().unwrap().handle_incoming_block(block);
            if let Err(e) = res {
                error!("error processing a bootstrap block: {:?}", e);
            }
            future::ok(blockchain)
        })
        .map(|_| ())
        .map_err(|e| {
            error!("bootstrap block streaming failed: {:?}", e);
        })
}
