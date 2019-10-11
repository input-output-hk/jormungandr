use super::{
    p2p::topology::{NodeData, NodeId},
    GlobalState, GlobalStateR,
};
use crate::{
    blockcfg::{Fragment, Header},
    intercom::{BlockMsg, TransactionMsg},
    settings::start::network::Configuration,
    utils::async_msg::MessageBox,
};
use futures::prelude::*;
use futures::sink;
use jormungandr_lib::interfaces::FragmentOrigin;
use network_core::error as core_error;
use network_core::gossip::Gossip;
use slog::Logger;

pub fn process_block_announcements<S>(
    inbound: S,
    node_id: NodeId,
    global_state: GlobalStateR,
    block_box: MessageBox<BlockMsg>,
    logger: Logger,
) where
    S: Stream<Item = Header, Error = core_error::Error> + Send + 'static,
{
    let state = global_state.clone();
    let stream_err_logger = logger.clone();
    let sink_err_logger = logger.clone();
    let stream = inbound
        .map_err(move |err| {
            debug!(
                stream_err_logger,
                "block subscription stream failure: {:?}", err
            );
        })
        .map(move |header| {
            state.peers.refresh_peer_on_block(node_id);
            BlockMsg::AnnouncedBlock(header, node_id)
        });
    global_state.spawn(
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
    );
}

pub fn process_block_announcement(
    header: Header,
    node_id: NodeId,
    global_state: &GlobalState,
    block_box: MessageBox<BlockMsg>,
) -> SendingBlockMsg {
    global_state.peers.refresh_peer_on_block(node_id);
    let future = block_box.send(BlockMsg::AnnouncedBlock(header, node_id));
    SendingBlockMsg { inner: future }
}

#[must_use = "futures do nothing unless polled"]
pub struct SendingBlockMsg {
    inner: sink::Send<MessageBox<BlockMsg>>,
}

impl Future for SendingBlockMsg {
    type Item = MessageBox<BlockMsg>;
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll().map_err(|_e| {
            core_error::Error::new(
                core_error::Code::Aborted,
                "the node stopped processing blocks",
            )
        })
    }
}

pub fn process_fragments<S>(
    inbound: S,
    node_id: NodeId,
    global_state: GlobalStateR,
    transaction_box: MessageBox<TransactionMsg>,
    logger: Logger,
) where
    S: Stream<Item = Fragment, Error = core_error::Error> + Send + 'static,
{
    let state = global_state.clone();
    let stream_err_logger = logger.clone();
    let sink_err_logger = logger.clone();
    let stream = inbound
        .map_err(move |err| {
            debug!(
                stream_err_logger,
                "fragment subscription stream failure: {:?}", err
            );
        })
        .map(move |fragment| {
            state.peers.refresh_peer_on_fragment(node_id);
            TransactionMsg::SendTransaction(FragmentOrigin::Network, vec![fragment])
        });
    global_state.spawn(
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
    );
}

pub fn process_gossip<S>(inbound: S, node_id: NodeId, global_state: GlobalStateR, logger: Logger)
where
    S: Stream<Item = Gossip<NodeData>, Error = core_error::Error> + Send + 'static,
{
    let state = global_state.clone();
    let err_logger = logger.clone();
    global_state.spawn(
        inbound
            .for_each(move |gossip| {
                trace!(logger, "received gossip: {:?}", gossip);
                let (nodes, filtered_out): (Vec<_>, Vec<_>) = gossip
                    .into_nodes()
                    .partition(|node| filter_gossip_node(node, &state.config));
                if filtered_out.len() > 0 {
                    debug!(logger, "nodes dropped from gossip: {:?}", filtered_out);
                }
                if !state.peers.refresh_peer_on_gossip(node_id) {
                    debug!(
                        logger,
                        "received gossip from node {} that is not in the peer map", node_id
                    );
                }
                state.topology.update(nodes);
                Ok(())
            })
            .map_err(move |err| {
                debug!(err_logger, "gossip subscription stream failure: {:?}", err);
            }),
    );
}

fn filter_gossip_node(node: &NodeData, config: &Configuration) -> bool {
    if config.allow_private_addresses {
        node.has_valid_address()
    } else {
        node.is_global()
    }
}
