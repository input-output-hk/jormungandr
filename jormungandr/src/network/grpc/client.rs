use crate::{
    blockcfg::{Block, HeaderHash},
    network::p2p::topology::NodeId,
    network::{BlockConfig, FetchBlockError},
    settings::start::network::Peer,
};
use futures::prelude::*;
use http::{HttpTryFrom, Uri};
use hyper::client::connect::{Destination, HttpConnector};
use network_core::client::{BlockService, Client as _};
use network_grpc::client::{Connect, ConnectFuture};
use slog::Logger;
use std::net::{IpAddr, SocketAddr};
use std::slice;
use tokio::{executor::DefaultExecutor, runtime};

pub type Connection = network_grpc::client::Connection<BlockConfig>;

pub fn connect(
    addr: SocketAddr,
    node_id: Option<NodeId>,
) -> ConnectFuture<BlockConfig, HttpConnector, DefaultExecutor> {
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
    info!(logger, "fetching block {} from {}", hash, peer.connection);
    let fetch = connect(peer.address(), None)
        .map_err(|err| FetchBlockError::Connect {
            source: Box::new(err),
        })
        .and_then(move |client: Connection| {
            client.ready().map_err(|err| FetchBlockError::Connect {
                source: Box::new(err),
            })
        })
        .and_then(move |mut client| {
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
