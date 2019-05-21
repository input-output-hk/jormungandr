use super::{
    p2p::topology::{Node, NodeId},
    GlobalStateR,
};
use crate::{blockcfg::Header, intercom::BlockMsg, utils::async_msg::MessageBox};
use futures::prelude::*;
use network_core::{error as core_error, gossip::Gossip};
use slog::Logger;

pub fn process_block_announcements<S>(
    inbound: S,
    node_id: NodeId,
    mut block_box: MessageBox<BlockMsg>,
    logger: Logger,
) -> tokio::executor::Spawn
where
    S: Stream<Item = Header, Error = core_error::Error> + Send + 'static,
{
    tokio::spawn(
        inbound
            .for_each(move |header| {
                block_box.send(BlockMsg::AnnouncedBlock(header, node_id));
                Ok(())
            })
            .map_err(move |err| {
                slog::info!(logger, "block subscription stream failure: {:?}", err);
            }),
    )
}

pub fn process_gossip<S>(inbound: S, state: GlobalStateR) -> tokio::executor::Spawn
where
    S: Stream<Item = Gossip<Node>, Error = core_error::Error> + Send + 'static,
{
    let err_logger = state.logger().clone();
    tokio::spawn(
        inbound
            .for_each(move |gossip| {
                slog::debug!(state.logger(), "received gossip: {:?}", gossip);
                state.topology.update(gossip.into_nodes());
                Ok(())
            })
            .map_err(move |err| {
                slog::info!(err_logger, "gossip subscription stream failure: {:?}", err);
            }),
    )
}
