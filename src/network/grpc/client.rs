use super::super::{GlobalState, NetworkBlockConfig};
use crate::settings::start::network::Peer;

use network_core::client::block::BlockService;
use network_grpc::{client::Client, peer as grpc_peer};

use futures::future;
use futures::prelude::*;
use tokio::{executor::DefaultExecutor, net::TcpStream};

pub fn run_connect_socket<B>(
    peer: Peer,
    _state: GlobalState<B>,
) -> impl Future<Item = (), Error = ()>
where
    B: NetworkBlockConfig,
{
    info!("connecting to subscription peer {}", peer.connection);
    let peer = grpc_peer::TcpPeer::new(*peer.address());
    let addr = peer.addr().clone();

    Client::connect(peer, DefaultExecutor::current())
        .map_err(move |err| {
            error!("Error connecting to peer {}: {:?}", addr, err);
        })
        .and_then(|mut client: Client<B, TcpStream, DefaultExecutor>| {
            client.subscribe_to_blocks().map_err(move |err| {
                error!("SubscribeToBlocks request failed: {:?}", err);
            })
        })
        .and_then(|_subscription| {
            // FIXME: do something to it
            future::ok(())
        })
}
