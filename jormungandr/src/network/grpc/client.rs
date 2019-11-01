use crate::{
    blockcfg::{Block, HeaderHash},
    network::{p2p::Id, BlockConfig},
    settings::start::network::Peer,
};
use futures::prelude::*;
use http::{HttpTryFrom, Uri};
use hyper::client::connect::{Destination, HttpConnector};
use network_core::client::{BlockService, Client as _};
use network_core::error as core_error;
use network_grpc::client::Connect;
use slog::Logger;
use thiserror::Error;
use tokio::runtime::{Runtime, TaskExecutor};

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::slice;

#[derive(Error, Debug)]
pub enum FetchBlockError {
    #[error("runtime initialization failed")]
    RuntimeInit { source: io::Error },
    #[error("connection to peer failed")]
    Connect { source: ConnectError },
    #[error("connection broken")]
    ClientNotReady { source: core_error::Error },
    #[error("block request failed")]
    GetBlocks { source: core_error::Error },
    #[error("block response stream failed")]
    GetBlocksStream { source: core_error::Error },
    #[error("no blocks received")]
    NoBlocks,
}

pub type Connection = network_grpc::client::Connection<BlockConfig>;
pub type ConnectFuture =
    network_grpc::client::ConnectFuture<BlockConfig, HttpConnector, TaskExecutor>;
pub type ConnectError = network_grpc::client::ConnectError<io::Error>;

pub fn connect(addr: SocketAddr, node_id: Option<Id>, executor: TaskExecutor) -> ConnectFuture {
    let uri = destination_uri(addr);
    let mut connector = HttpConnector::new(2);
    connector.set_nodelay(true);
    let mut builder = Connect::new(connector, executor);
    if let Some(id) = node_id {
        builder.node_id(id);
    }
    builder.connect(Destination::try_from_uri(uri).unwrap())
}

fn destination_uri(addr: SocketAddr) -> Uri {
    let ip = addr.ip();
    let uri = match ip {
        IpAddr::V4(ip) => format!("http://{}:{}", ip, addr.port()),
        IpAddr::V6(ip) => format!("http://[{}]:{}", ip, addr.port()),
    };
    HttpTryFrom::try_from(uri).unwrap()
}

// Fetches a block from a network peer in a one-off, blocking call.
// This function is used during node bootstrap to fetch the genesis block.
pub fn fetch_block(
    peer: Peer,
    hash: HeaderHash,
    logger: &Logger,
) -> Result<Block, FetchBlockError> {
    info!(logger, "fetching block {}", hash);
    let runtime = Runtime::new().map_err(|e| FetchBlockError::RuntimeInit { source: e })?;
    let fetch = connect(peer.address(), None, runtime.executor())
        .map_err(|err| FetchBlockError::Connect { source: err })
        .and_then(move |client: Connection| {
            client
                .ready()
                .map_err(|err| FetchBlockError::ClientNotReady { source: err })
        })
        .and_then(move |mut client| {
            client
                .get_blocks(slice::from_ref(&hash))
                .map_err(|err| FetchBlockError::GetBlocks { source: err })
        })
        .and_then(move |stream| {
            stream
                .into_future()
                .map_err(|(err, _)| FetchBlockError::GetBlocksStream { source: err })
        })
        .and_then(|(maybe_block, _)| match maybe_block {
            None => Err(FetchBlockError::NoBlocks),
            Some(block) => Ok(block),
        });
    runtime.block_on_all(fetch)
}
