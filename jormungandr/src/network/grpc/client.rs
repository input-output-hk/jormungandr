use super::origin_authority;
use crate::{
    blockcfg::{Block, HeaderHash},
    network::{BlockConfig, ConnectionState, FetchBlockError},
    settings::start::network::Peer,
};
use futures::prelude::*;
use http::uri;
use network_core::{client::block::BlockService, gossip::Node};
use network_grpc::client::{Connect, ConnectFuture, TcpConnector};
use slog::Logger;
use std::{net::SocketAddr, slice};
use tokio::{executor::DefaultExecutor, runtime};

pub type Connection = network_grpc::client::Connection<BlockConfig>;

pub fn connect(
    state: &ConnectionState,
) -> ConnectFuture<BlockConfig, SocketAddr, TcpConnector, DefaultExecutor> {
    let addr = state.connection;
    let origin = origin_authority(addr);
    Connect::new(TcpConnector, DefaultExecutor::current())
        .origin(uri::Scheme::HTTP, origin)
        .node_id(state.global.node.id().clone())
        .connect(addr)
}

// Fetches a block from a network peer in a one-off, blocking call.
// This function is used during node bootstrap to fetch the genesis block.
pub fn fetch_block(
    peer: Peer,
    hash: &HeaderHash,
    logger: &Logger,
) -> Result<Block, FetchBlockError> {
    info!(logger, "fetching block {} from {}", hash, peer.connection);
    let addr = peer.address();
    let origin = origin_authority(addr);
    let fetch = Connect::new(TcpConnector, DefaultExecutor::current())
        .origin(uri::Scheme::HTTP, origin)
        .connect(addr)
        .map_err(|err| FetchBlockError::Connect {
            source: Box::new(err),
        })
        .and_then(move |mut client: Connection| {
            client
                .get_blocks(slice::from_ref(hash))
                .map_err(|err| FetchBlockError::GetBlocks { source: err })
        })
        .and_then(move |stream| {
            stream
                .into_future()
                .map_err(|(err, _)| FetchBlockError::GetBlocks { source: err })
        })
        .and_then(|(maybe_block, _)| match maybe_block {
            None => Err(FetchBlockError::NoBlocks),
            Some(block) => Ok(block),
        });
    runtime::current_thread::block_on_all(fetch)
}
