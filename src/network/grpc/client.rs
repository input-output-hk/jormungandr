use super::super::{propagate, BlockConfig, ConnectionState, GlobalState};
use crate::{intercom::BlockMsg, settings::start::network::Peer};

use network_core::client::block::BlockService;
use network_grpc::{client::Client, peer as grpc_peer};

use bytes::Bytes;
use futures::future;
use futures::prelude::*;
use tokio::executor::DefaultExecutor;

pub fn run_connect_socket(
    peer: Peer,
    state: GlobalState,
) -> (impl Future<Item = (), Error = ()>, propagate::PeerHandlesR) {
    let state = ConnectionState::new_peer(&state, &peer);

    info!("connecting to subscription peer {}", peer.connection);
    info!("address: {}", peer.address());
    let peer = grpc_peer::TcpPeer::new(*peer.address());
    let addr = peer.addr().clone();
    let authority =
        http::uri::Authority::from_shared(Bytes::from(format!("{}:{}", addr.ip(), addr.port())))
            .unwrap();
    let origin = http::uri::Builder::new()
        .scheme("http")
        .authority(authority)
        .path_and_query("/")
        .build()
        .unwrap();
    let propagation = state.propagation.clone();
    let mut block_box = state.channels.block_box;

    let fut = Client::connect(peer, DefaultExecutor::current(), origin)
        .map_err(move |err| {
            error!("Error connecting to peer {}: {:?}", addr, err);
        })
        .and_then(move |mut client: Client<BlockConfig, _, _>| {
            let mut sub_handles = propagation.lock().unwrap();
            client
                .block_subscription(sub_handles.blocks.subscribe())
                .map_err(move |err| {
                    error!("BlockSubscription request failed: {:?}", err);
                })
        })
        .and_then(move |subscription| {
            subscription
                .for_each(move |header| {
                    block_box.send(BlockMsg::AnnouncedBlock(header));
                    future::ok(())
                })
                .map_err(|err| {
                    error!("Block subscription failed: {:?}", err);
                })
        });

    (fut, state.propagation)
}
