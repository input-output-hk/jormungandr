use super::{
    super::{propagate, subscription, BlockConfig, Channels, ConnectionState, GlobalStateR},
    origin_authority,
};

use network_core::{
    client::{block::BlockService, gossip::GossipService},
    gossip::Node,
};
use network_grpc::{
    client::{Connect, Connection},
    peer as grpc_peer,
};

use futures::prelude::*;
use http::uri;
use tokio::{executor::DefaultExecutor, net::TcpStream};
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

    Connect::new(peer, DefaultExecutor::current())
        .origin(uri::Scheme::HTTP, origin)
        .node_id(state.global.node.id().clone())
        .call(())
        .map_err(move |err| {
            error!("Error connecting to peer {}: {:?}", addr, err);
        })
        .and_then(move |client| subscribe(client, state.global, channels))
}

fn subscribe(
    mut client: Connection<BlockConfig, TcpStream, DefaultExecutor>,
    global_state: GlobalStateR,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
    let block_box = channels.block_box;
    let mut prop_handles = propagate::PeerHandles::new();
    let block_sub = client.block_subscription(prop_handles.blocks.subscribe());
    let gossip_sub = client.gossip_subscription(prop_handles.gossip.subscribe());
    block_sub
        .join(gossip_sub)
        .map_err(move |err| {
            error!("Subscription request failed: {:?}", err);
        })
        .and_then(move |((block_sub, node_id), (gossip_sub, node_id_1))| {
            if node_id != node_id_1 {
                warn!(
                    "peer subscription IDs do not match: {} != {}",
                    node_id, node_id_1
                );
                return Err(());
            }
            global_state
                .propagation_peers
                .insert_peer(node_id, prop_handles);
            subscription::process_blocks(block_sub, block_box);
            subscription::process_gossip(gossip_sub, global_state);
            Ok(())
        })
}
