use super::super::BlockConfig;
use super::origin_authority;
use crate::{blockchain::BlockchainR, settings::start::network::Peer};
use blockcfg::Block;

use chain_core::property::HasHeader;
use network_core::client::block::BlockService as _;
use network_grpc::client::{Connect, Connection, TcpConnector};

use http::uri;
use tokio::prelude::*;
use tokio::{executor::DefaultExecutor, runtime::current_thread};

use std::fmt::Debug;

pub fn bootstrap_from_peer(peer: Peer, blockchain: BlockchainR) {
    info!("connecting to bootstrap peer {}", peer.connection);
    let addr = peer.address();
    let origin = origin_authority(addr);
    let bootstrap = Connect::new(TcpConnector, DefaultExecutor::current())
        .origin(uri::Scheme::HTTP, origin)
        .connect(addr)
        .map_err(|e| {
            error!("failed to connect to bootstrap peer: {:?}", e);
        })
        .and_then(|mut client: Connection<BlockConfig, _, _>| {
            let tip = blockchain.lock_read().get_tip().unwrap();
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
            use crate::blockchain::handle_block;
            debug!(
                "received block from the bootstrap node: {:#?}",
                block.header()
            );
            let res = handle_block(&mut blockchain.lock_write(), block, true);
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
