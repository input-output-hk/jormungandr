use crate::{
    blockcfg::{Block, HeaderHash},
    network::convert::Decode,
    network::{concurrency_limits, keepalive_durations},
    settings::start::network::{Peer, Protocol},
};
use chain_network::data as net_data;
use chain_network::error as net_error;
use chain_network::grpc::client::Builder;
use futures::prelude::*;
use thiserror::Error;
use tonic::transport;

use std::convert::TryFrom;
use std::net::SocketAddr;

pub use chain_network::grpc::client::{
    BlockSubscription, FragmentSubscription, GossipSubscription,
};

#[derive(Error, Debug)]
pub enum FetchBlockError {
    #[error("connection to peer failed")]
    Connect { source: ConnectError },
    #[error("block request failed")]
    GetBlocks { source: net_error::Error },
    #[error("block response stream failed")]
    GetBlocksStream { source: net_error::Error },
    #[error("no blocks received")]
    NoBlocks,
    #[error("Unexpected block hash: requested {requested} received {received}")]
    UnexpectedBlock {
        requested: HeaderHash,
        received: HeaderHash,
    },
}

pub type ConnectError = transport::Error;

pub type Client = chain_network::grpc::Client<tonic::transport::Channel>;

pub async fn connect(peer: &Peer) -> Result<Client, ConnectError> {
    connect_internal(peer, Builder::new()).await
}

async fn connect_internal(peer: &Peer, builder: Builder) -> Result<Client, ConnectError> {
    assert!(peer.protocol == Protocol::Grpc);
    let endpoint = destination_endpoint(peer.connection)
        .concurrency_limit(concurrency_limits::CLIENT_REQUESTS)
        .tcp_keepalive(Some(keepalive_durations::TCP))
        .http2_keep_alive_interval(keepalive_durations::HTTP2)
        .timeout(peer.timeout);
    builder.connect(endpoint).await
}

fn destination_endpoint(addr: SocketAddr) -> transport::Endpoint {
    let uri = format!("http://{}", addr);
    transport::Endpoint::try_from(uri).unwrap()
}

// Fetches a block from a network peer.
// This function is used during node bootstrap to fetch the genesis block.
pub async fn fetch_block(peer: &Peer, hash: HeaderHash) -> Result<Block, FetchBlockError> {
    tracing::info!("fetching block {}", hash);
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

            if block.header.id() == hash {
                Ok(block)
            } else {
                Err(FetchBlockError::UnexpectedBlock {
                    requested: hash.to_owned(),
                    received: block.header.id(),
                })
            }
        }
        None => Err(FetchBlockError::NoBlocks),
        Some(Err(e)) => Err(FetchBlockError::GetBlocksStream { source: e }),
    }
}
