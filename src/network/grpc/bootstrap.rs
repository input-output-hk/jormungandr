use crate::blockcfg::BlockConfig;
use crate::blockchain::BlockchainR;

use chain_core::property::{self, Deserialize, HasHeader};
use network_core::client::block::BlockService;
use network_grpc::client::Client;

use tokio::prelude::*;
use tokio::{executor::DefaultExecutor, runtime::current_thread};

use std::fmt::Debug;

pub fn bootstrap_from_target<P, B>(peer: P, blockchain: BlockchainR<B>)
where
    B: BlockConfig,
    P: tower_service::Service<(), Error = std::io::Error> + 'static,
    <P as tower_service::Service<()>>::Response:
        tokio::io::AsyncWrite + tokio::io::AsyncRead + 'static + Send,
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
        .and_then(
            |mut client: Client<
                B::Block,
                B::Gossip,
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
