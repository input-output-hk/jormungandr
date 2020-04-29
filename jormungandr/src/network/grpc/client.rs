use crate::{
    blockcfg::{Block, HeaderHash},
    network::concurrency_limits,
    network::convert::Decode,
    settings::start::network::{Peer, Protocol},
};
use chain_network::data as net_data;
use chain_network::error as net_error;
use futures03::prelude::*;
use slog::Logger;
use thiserror::Error;
use tonic::transport;

use std::convert::TryFrom;
use std::net::{IpAddr, SocketAddr};

pub use chain_network::grpc::client::{
    BlockSubscription, FragmentSubscription, GossipSubscription,
};

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

pub type Client = chain_network::grpc::Client<tonic::transport::Channel>;

pub async fn connect(peer: &Peer) -> Result<Client, ConnectError> {
    assert!(peer.protocol == Protocol::Grpc);
    let endpoint = destination_endpoint(peer.connection)
        .concurrency_limit(concurrency_limits::CLIENT_REQUESTS)
        .timeout(peer.timeout);
    Client::connect(endpoint).await
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
    let mut client = connect(peer)
        .await
        .map_err(|err| FetchBlockError::Connect { source: err })?;
    let block_id = net_data::BlockId::try_from(hash.as_bytes()).unwrap();
    let stream = client
        .get_blocks(vec![block_id].into())
        .await
        .map_err(|err| FetchBlockError::GetBlocks { source: err })?;
    let (next_block, _) = stream.into_future().await;
    match next_block {
        Some(Ok(block)) => {
            let block = block
                .decode()
                .map_err(|e| FetchBlockError::GetBlocksStream { source: e })?;
            Ok(block)
        }
        None => Err(FetchBlockError::NoBlocks),
        Some(Err(e)) => Err(FetchBlockError::GetBlocksStream { source: e }),
    }
}
