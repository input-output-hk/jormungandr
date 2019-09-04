use super::{
    p2p::topology::{Node, NodeId},
    GlobalState, GlobalStateR,
};
use crate::{
    blockcfg::{Fragment, Header},
    intercom::BlockMsg,
    utils::async_msg::MessageBox,
};
use futures::prelude::*;
use network_core::{error as core_error, gossip::Gossip};
use slog::Logger;

pub fn process_block_announcements<S>(
    inbound: S,
    node_id: NodeId,
    global_state: GlobalStateR,
    mut block_box: MessageBox<BlockMsg>,
    logger: Logger,
) -> tokio::executor::Spawn
where
    S: Stream<Item = Header, Error = core_error::Error> + Send + 'static,
{
    tokio::spawn(
        inbound
            .for_each(move |header| {
                process_block_announcement(header, node_id, &global_state, &mut block_box);
                Ok(())
            })
            .map_err(move |err| {
                info!(logger, "block subscription stream failure: {:?}", err);
            }),
    )
}

pub fn process_block_announcement(
    header: Header,
    node_id: NodeId,
    global_state: &GlobalState,
    block_box: &mut MessageBox<BlockMsg>,
) {
    global_state.peers.bump_peer_for_block_fetch(node_id);
    block_box
        .try_send(BlockMsg::AnnouncedBlock(header, node_id))
        .unwrap();
}

pub fn process_fragments<S>(
    inbound: S,
    state: GlobalStateR,
    logger: Logger,
) -> tokio::executor::Spawn
where
    S: Stream<Item = Fragment, Error = core_error::Error> + Send + 'static,
{
    let err_logger = logger.clone();
    tokio::spawn(
        inbound
            .for_each(move |fragment| {
                // TODO: implement
                Ok(())
            })
            .map_err(move |err| {
                info!(
                    err_logger,
                    "fragment subscription stream failure: {:?}", err
                );
            }),
    )
}

pub fn process_gossip<S>(inbound: S, state: GlobalStateR, logger: Logger) -> tokio::executor::Spawn
where
    S: Stream<Item = Gossip<Node>, Error = core_error::Error> + Send + 'static,
{
    let err_logger = logger.clone();
    tokio::spawn(
        inbound
            .for_each(move |gossip| {
                debug!(logger, "received gossip: {:?}", gossip);
                state.topology.update(gossip.into_nodes());
                Ok(())
            })
            .map_err(move |err| {
                info!(err_logger, "gossip subscription stream failure: {:?}", err);
            }),
    )
}
