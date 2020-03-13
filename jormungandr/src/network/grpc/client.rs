use crate::{
    blockcfg::{Block, HeaderHash},
    network::concurrency_limits,
    network::p2p::Id,
    settings::start::network::{Peer, Protocol},
};
use chain_network::error as net_error;
pub use chain_network::grpc::Client;
use futures::prelude::*;
use slog::Logger;
use thiserror::Error;
use tonic::transport;

use std::net::{IpAddr, SocketAddr};
use std::slice;

#[derive(Error, Debug)]
pub enum FetchBlockError {
    #[error("connection to peer failed")]
    Connect { source: ConnectError },
    #[error("connection broken")]
    ClientNotReady { source: net_error::Error },
    #[error("block request failed")]
    GetBlocks { source: net_error::Error },
    #[error("block response stream failed")]
    GetBlocksStream { source: net_error::Error },
    #[error("no blocks received")]
    NoBlocks,
}

pub type ConnectError = transport::Error;

pub async fn connect(peer: &Peer, node_id: Option<Id>) -> Result<Client, ConnectError> {
    assert!(peer.protocol == Protocol::Grpc);
    let endpoint = destination_endpoint(peer.connection);
    endpoint.concurrency_limit(concurrency_limits::CLIENT_REQUESTS);
    endpoint.timeout(peer.timeout);
    Client::connect(endpoint)
}

fn destination_endpoint(addr: SocketAddr) -> transport::Endpoint {
    let ip = addr.ip();
    let uri = match ip {
        IpAddr::V4(ip) => format!("http://{}:{}", ip, addr.port()),
        IpAddr::V6(ip) => format!("http://[{}]:{}", ip, addr.port()),
    };
    transport::Endpoint::try_from(uri).unwrap()
}

// Fetches a block from a network peer.
// This function is used during node bootstrap to fetch the genesis block.
pub async fn fetch_block(
    peer: &Peer,
    hash: HeaderHash,
    logger: &Logger,
) -> Result<Block, FetchBlockError> {
    info!(logger, "fetching block {}", hash);
    let client = connect(peer, None)
        .await
        .map_err(|err| FetchBlockError::Connect { source: err })?;
    client
        .ready()
        .await
        .map_err(|err| FetchBlockError::ClientNotReady { source: err })?;
    let stream = client
        .get_blocks(slice::from_ref(&hash))
        .await
        .map_err(|err| FetchBlockError::GetBlocks { source: err })?;
    let (maybe_block, _) = stream
        .into_future()
        .await
        .map_err(|(err, _)| FetchBlockError::GetBlocksStream { source: err })?;

    match maybe_block {
        None => Err(FetchBlockError::NoBlocks),
        Some(block) => Ok(block),
    }
}
