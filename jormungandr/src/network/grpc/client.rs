use crate::{
    blockcfg::{Block, HeaderHash},
    network::p2p::topology::NodeId,
    network::BlockConfig,
    settings::start::network::Peer,
};
use futures::prelude::*;
use http::{HttpTryFrom, Uri};
use hyper::client::connect::{Destination, HttpConnector};
use network_core::client::{BlockService, Client as _};
use network_core::error as core_error;
use network_grpc::client::{Connect, ConnectError};
use slog::Logger;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::slice;
use std::{error::Error, fmt};
use tokio::{executor::DefaultExecutor, runtime};

#[derive(Debug)]
pub enum FetchBlockError {
    Connect { source: ConnectError<io::Error> },
    Ready { source: core_error::Error },
    GetBlocks { source: core_error::Error },
    GetBlocksStream { source: core_error::Error },
    NoBlocks,
}

impl fmt::Display for FetchBlockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FetchBlockError::Connect { .. } => write!(f, "connection to peer failed"),
            FetchBlockError::Ready { .. } => write!(f, "gRPC client is not ready to send request"),
            FetchBlockError::GetBlocks { .. } => write!(f, "block request failed"),
            FetchBlockError::GetBlocksStream { .. } => write!(f, "block response stream failed"),
            FetchBlockError::NoBlocks => write!(f, "no blocks in the stream"),
        }
    }
}

impl Error for FetchBlockError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FetchBlockError::Connect { source } => Some(source),
            FetchBlockError::Ready { source } => Some(source),
            FetchBlockError::GetBlocks { source } => Some(source),
            FetchBlockError::GetBlocksStream { source } => Some(source),
            FetchBlockError::NoBlocks => None,
        }
    }
}

pub type Connection = network_grpc::client::Connection<BlockConfig>;
pub type ConnectFuture =
    network_grpc::client::ConnectFuture<BlockConfig, HttpConnector, DefaultExecutor>;

pub fn connect(addr: SocketAddr, node_id: Option<NodeId>) -> ConnectFuture {
    let uri = destination_uri(addr);
    let mut connector = HttpConnector::new(2);
    connector.set_nodelay(true);
    let mut builder = Connect::new(connector, DefaultExecutor::current());
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
    hash: &HeaderHash,
    logger: &Logger,
) -> Result<Block, FetchBlockError> {
    info!(logger, "fetching block {}", hash);
    let fetch = connect(peer.address(), None)
        .map_err(|err| FetchBlockError::Connect { source: err })
        .and_then(move |client: Connection| {
            client
                .ready()
                .map_err(|err| FetchBlockError::Ready { source: err })
        })
        .and_then(move |mut client| {
            client
                .get_blocks(slice::from_ref(hash))
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
    runtime::current_thread::block_on_all(fetch)
}
