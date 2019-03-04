use super::super::GlobalState;
use crate::{blockcfg::BlockConfig, settings::start::network::Peer};

use chain_core::property;
use network_core::client::block::BlockService;
use network_grpc::{
    client::{chain_bounds, Client},
    peer as grpc_peer,
};

use futures::future;
use futures::prelude::*;
use tokio::{executor::DefaultExecutor, net::TcpStream};

use std::str::FromStr;

pub fn run_connect_socket<B>(peer: Peer, _state: GlobalState<B>) -> impl Future<Item = (), Error = ()>
where
    B: BlockConfig,
    B::Block: chain_bounds::Block,
    <B::Block as property::Block>::Date: FromStr,
{
    info!("connecting to subscription peer {}", peer.connection);
    let peer = grpc_peer::TcpPeer::new(*peer.address());
    let addr = peer.addr().clone();

    Client::connect(peer, DefaultExecutor::current())
        .map_err(move |err| {
            error!("Error connecting to peer {}: {:?}", addr, err);
        })
        .and_then(|mut client: Client<B::Block, TcpStream, DefaultExecutor>| {
            client.subscribe_to_blocks().map_err(move |err| {
                error!("SubscribeToBlocks request failed: {:?}", err);
            })
        })
        .and_then(|_subscription| {
            // FIXME: do something to it
            future::ok(())
        })
}
