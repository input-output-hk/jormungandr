use super::{
    p2p::topology::{Node, NodeId},
    GlobalState, GlobalStateR,
};
use crate::{
    blockcfg::{Fragment, Header},
    intercom::{BlockMsg, TransactionMsg},
    settings::start::network::Configuration,
    utils::async_msg::MessageBox,
};
use futures::prelude::*;
use jormungandr_lib::interfaces::FragmentOrigin;
use network_core::{error as core_error, gossip::Gossip};
use slog::Logger;

pub fn process_block_announcements<S>(
    inbound: S,
    node_id: NodeId,
    global_state: GlobalStateR,
    block_box: MessageBox<BlockMsg>,
    logger: Logger,
) -> tokio::executor::Spawn
where
    S: Stream<Item = Header, Error = core_error::Error> + Send + 'static,
{
    let stream_err_logger = logger.clone();
    let sink_err_logger = logger.clone();
    let stream = inbound
        .map_err(move |err| {
            info!(
                stream_err_logger,
                "block subscription stream failure: {:?}", err
            );
        })
        .map(move |header| {
            global_state.peers.bump_peer_for_block_fetch(node_id);
            BlockMsg::AnnouncedBlock(header, node_id)
        });
    tokio::spawn(
        block_box
            .sink_map_err(move |_| {
                error!(
                    sink_err_logger,
                    "failed to send block announcement to the block task"
                );
            })
            .send_all(stream)
            .map(move |_| {
                debug!(logger, "block subscription ended");
            }),
    )
}

// TODO: convert this function and all uses of it to async
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
    _state: GlobalStateR,
    transaction_box: MessageBox<TransactionMsg>,
    logger: Logger,
) -> tokio::executor::Spawn
where
    S: Stream<Item = Fragment, Error = core_error::Error> + Send + 'static,
{
    let stream_err_logger = logger.clone();
    let sink_err_logger = logger.clone();
    let stream = inbound
        .map_err(move |err| {
            info!(
                stream_err_logger,
                "fragment subscription stream failure: {:?}", err
            );
        })
        .map(|fragment| TransactionMsg::SendTransaction(FragmentOrigin::Network, vec![fragment]));
    tokio::spawn(
        transaction_box
            .sink_map_err(move |_| {
                error!(
                    sink_err_logger,
                    "failed to send fragment to the transaction task"
                );
            })
            .send_all(stream)
            .map(move |_| {
                debug!(logger, "fragment subscription ended");
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
                trace!(logger, "received gossip: {:?}", gossip);
                let nodes = gossip
                    .into_nodes()
                    .filter(|node| filter_gossip_node(node, &state.config));
                state.topology.update(nodes);
                Ok(())
            })
            .map_err(move |err| {
                info!(err_logger, "gossip subscription stream failure: {:?}", err);
            }),
    )
}

fn filter_gossip_node(node: &Node, config: &Configuration) -> bool {
    if config.allow_private_addresses {
        node.has_valid_address()
    } else {
        node.is_global()
    }
}
