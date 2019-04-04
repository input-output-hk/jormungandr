use super::{
    super::{BlockConfig, Channels, ConnectionState},
    origin_uri,
};
use crate::intercom::BlockMsg;

use network_core::client::block::BlockService;
use network_grpc::{client::Client, peer as grpc_peer};

use futures::future;
use futures::prelude::*;
use tokio::executor::DefaultExecutor;

use std::net::SocketAddr;

pub fn run_connect_socket(
    addr: SocketAddr,
    state: ConnectionState,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
    info!("connecting to subscription peer {}", state.connection);
    info!("address: {}", addr);
    let peer = grpc_peer::TcpPeer::new(addr);
    let origin = origin_uri(addr);
    let mut block_box = channels.block_box;

    Client::connect(peer, DefaultExecutor::current(), origin)
        .map_err(move |err| {
            error!("Error connecting to peer {}: {:?}", addr, err);
        })
        .and_then(move |mut client: Client<BlockConfig, _, _>| {
            let mut sub_handles = state.propagation.lock().unwrap();
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
        })
}
