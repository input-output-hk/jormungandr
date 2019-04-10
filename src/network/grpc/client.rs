use super::{
    super::{BlockConfig, Channels, ConnectionState},
    origin_authority,
};
use crate::intercom::BlockMsg;

use network_core::{client::block::BlockService, gossip::Node};
use network_grpc::{
    client::{Connect, Connection},
    peer as grpc_peer,
};

use futures::future;
use futures::prelude::*;
use http::uri;
use tokio::executor::DefaultExecutor;
use tower_service::Service as _;

use std::net::SocketAddr;

pub fn run_connect_socket(
    addr: SocketAddr,
    state: ConnectionState,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
    info!("connecting to subscription peer {}", state.connection);
    info!("address: {}", addr);
    let peer = grpc_peer::TcpPeer::new(addr);
    let origin = origin_authority(addr);
    let mut block_box = channels.block_box;

    Connect::new(peer, DefaultExecutor::current())
        .origin(uri::Scheme::HTTP, origin)
        .node_id(state.global.node.id().clone())
        .call(())
        .map_err(move |err| {
            error!("Error connecting to peer {}: {:?}", addr, err);
        })
        .and_then(move |mut client: Connection<BlockConfig, _, _>| {
            let mut sub_handles = state.propagation.lock().unwrap();
            client
                .block_subscription(sub_handles.blocks.subscribe())
                .map_err(move |err| {
                    error!("BlockSubscription request failed: {:?}", err);
                })
        })
        .and_then(move |(subscription, _node_id)| {
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
