use super::origin_authority;
use crate::{
    blockcfg::{Block, HeaderHash},
    intercom::BlockMsg,
    network::{
        p2p_topology as p2p, propagate, subscription, BlockConfig, Channels, ConnectionState,
        FetchBlockError, GlobalStateR,
    },
    settings::start::network::Peer,
    utils::async_msg::MessageBox,
};

use network_core::{
    client::{block::BlockService, gossip::GossipService},
    gossip::Node,
    subscription::BlockEvent,
};
use network_grpc::{
    client::{Connect, Connection},
    peer as grpc_peer,
};

use futures::prelude::*;
use http::uri;
use tokio::{executor::DefaultExecutor, net::TcpStream, runtime};
use tower_service::Service as _;

use std::slice;

pub fn connect(
    state: ConnectionState,
    channels: Channels,
) -> impl Future<Item = (p2p::NodeId, propagate::PeerHandles), Error = ()> {
    info!("connecting to subscription peer {}", state.connection);
    let addr = state.connection;
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
) -> impl Future<Item = (p2p::NodeId, propagate::PeerHandles), Error = ()> {
    let block_box = channels.block_box;
    let mut prop_handles = propagate::PeerHandles::new();
    let block_req = client.block_subscription(prop_handles.blocks.subscribe());
    let gossip_req = client.gossip_subscription(prop_handles.gossip.subscribe());
    block_req
        .join(gossip_req)
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
            let block_sub = block_sub.map(|event| match event {
                BlockEvent::Announce(header) => header,
                BlockEvent::Solicit(_) => {
                    // TODO: fetch blocks from client request task
                    // and upload them
                    unimplemented!()
                }
            });
            subscription::process_block_announcements(node_id, block_sub, block_box.clone());
            subscription::process_gossip(gossip_sub, global_state);
            process_block_solicitations(client, &mut prop_handles, block_box);
            Ok((node_id, prop_handles))
        })
}

fn process_block_solicitations(
    mut client: Connection<BlockConfig, TcpStream, DefaultExecutor>,
    prop_handles: &mut propagate::PeerHandles,
    block_box: MessageBox<BlockMsg>,
) {
    tokio::spawn(
        prop_handles
            .solicit_blocks
            .subscribe()
            .for_each(move |block_ids| {
                let block_box = block_box.clone();
                client.get_blocks(&block_ids).and_then(move |blocks| {
                    let mut block_box = block_box.clone();
                    blocks.for_each(move |block| {
                        block_box.send(BlockMsg::NetworkBlock(block));
                        Ok(())
                    })
                })
            })
            .map_err(|e| {
                info!("block solicitation failed: {:?}", e);
            }),
    );
}

// Fetches a block from a network peer in a one-off, blocking call.
// This function is used during node bootstrap to fetch the genesis block.
pub fn fetch_block(peer: Peer, hash: &HeaderHash) -> Result<Block, FetchBlockError> {
    info!("fetching block {} from {}", hash, peer.connection);
    let addr = peer.address();
    let origin = origin_authority(addr);
    let peer = grpc_peer::TcpPeer::new(addr);
    let fetch = Connect::new(peer, DefaultExecutor::current())
        .origin(uri::Scheme::HTTP, origin)
        .call(())
        .map_err(|err| FetchBlockError::Connect {
            source: Box::new(err),
        })
        .and_then(move |mut client: Connection<BlockConfig, _, _>| {
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
