use super::{
    p2p::topology::{NodeData, NodeId},
    GlobalStateR,
};
use crate::{
    blockcfg::{Fragment, Header},
    intercom::{BlockMsg, TransactionMsg},
    log::stream::Log,
    settings::start::network::Configuration,
    utils::async_msg::MessageBox,
};
use futures::prelude::*;
use jormungandr_lib::interfaces::FragmentOrigin;
use network_core::error as core_error;
use network_core::gossip::{Gossip, Node as _};
use slog::Logger;

pub fn process_block_announcements<S>(
    inbound: S,
    node_id: NodeId,
    global_state: GlobalStateR,
    block_box: MessageBox<BlockMsg>,
    logger: &Logger,
) where
    S: Stream<Item = Header, Error = core_error::Error> + Send + 'static,
{
    let sink = BlockAnnouncementProcessor::new(block_box, node_id, global_state.clone(), logger);
    let logger = sink.logger.clone();
    let inspect_logger = logger.clone();
    let stream_err_logger = logger.clone();
    let stream = inbound
        .map_err(move |e| {
            debug!(
                stream_err_logger,
                "block subscription stream failure";
                "error" => ?e,
            );
        })
        .trace(logger.clone(), "received header of announced block")
        .inspect(move |header| {
            info!(inspect_logger, "received block announcement"; "hash" => %header.hash());
        });
    global_state.spawn(sink.send_all(stream).map(move |_| {
        debug!(logger, "block subscription ended by the peer");
    }));
}

pub fn process_fragments<S>(
    inbound: S,
    node_id: NodeId,
    global_state: GlobalStateR,
    fragment_box: MessageBox<TransactionMsg>,
    logger: &Logger,
) where
    S: Stream<Item = Fragment, Error = core_error::Error> + Send + 'static,
{
    let sink = FragmentProcessor::new(fragment_box, node_id, global_state.clone(), logger);
    let logger = sink.logger.clone();
    let stream_err_logger = logger.clone();
    let stream = inbound
        .map_err(move |e| {
            debug!(
                stream_err_logger,
                "fragment subscription stream failure";
                "error" => ?e,
            );
        })
        .trace(logger.clone(), "received fragment")
        // TODO: chunkify the fragment stream non-greedily
        .map(|fragment| vec![fragment]);
    global_state.spawn(sink.send_all(stream).map(move |_| {
        debug!(logger, "fragment subscription ended by the peer");
    }));
}

pub fn process_gossip<S>(inbound: S, node_id: NodeId, global_state: GlobalStateR, logger: &Logger)
where
    S: Stream<Item = Gossip<NodeData>, Error = core_error::Error> + Send + 'static,
{
    let processor = GossipProcessor::new(node_id, global_state.clone(), logger);
    let logger = processor.logger.clone();
    let err_logger = logger.clone();
    global_state.spawn(
        inbound
            .map_err(move |e| {
                debug!(
                    err_logger,
                    "gossip subscription stream failure";
                    "error" => ?e,
                );
            })
            .trace(logger.clone(), "received gossip")
            .for_each(move |gossip| {
                processor.process_item(gossip);
                Ok(())
            })
            .map(move |_| {
                debug!(logger, "gossip subscription ended by the peer");
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

#[must_use = "sinks do nothing unless polled"]
pub struct BlockAnnouncementProcessor {
    mbox: MessageBox<BlockMsg>,
    node_id: NodeId,
    global_state: GlobalStateR,
    logger: Logger,
}

impl BlockAnnouncementProcessor {
    pub fn new(
        mbox: MessageBox<BlockMsg>,
        node_id: NodeId,
        global_state: GlobalStateR,
        logger: &Logger,
    ) -> Self {
        let logger = logger.new(o!("stream" => "block_events", "direction" => "in"));
        BlockAnnouncementProcessor {
            mbox,
            node_id,
            global_state,
            logger,
        }
    }

    pub fn message_box(&self) -> MessageBox<BlockMsg> {
        self.mbox.clone()
    }
}

#[must_use = "sinks do nothing unless polled"]
pub struct FragmentProcessor {
    mbox: MessageBox<TransactionMsg>,
    node_id: NodeId,
    global_state: GlobalStateR,
    logger: Logger,
}

impl FragmentProcessor {
    pub fn new(
        mbox: MessageBox<TransactionMsg>,
        node_id: NodeId,
        global_state: GlobalStateR,
        logger: &Logger,
    ) -> Self {
        let logger = logger.new(o!("stream" => "fragments", "direction" => "in"));
        FragmentProcessor {
            mbox,
            node_id,
            global_state,
            logger,
        }
    }
}

pub struct GossipProcessor {
    node_id: NodeId,
    global_state: GlobalStateR,
    logger: Logger,
}

impl GossipProcessor {
    pub fn new(node_id: NodeId, global_state: GlobalStateR, logger: &Logger) -> Self {
        let logger = logger.new(o!("stream" => "gossip", "direction" => "in"));
        GossipProcessor {
            node_id,
            global_state,
            logger,
        }
    }

    pub fn process_item(&self, gossip: Gossip<NodeData>) {
        let (nodes, filtered_out): (Vec<_>, Vec<_>) = gossip.into_nodes().partition(|node| {
            filter_gossip_node(node, &self.global_state.config)
                || (node.id() == self.node_id && node.address().is_none())
        });
        if filtered_out.len() > 0 {
            debug!(self.logger, "nodes dropped from gossip: {:?}", filtered_out);
        }
        if !self.global_state.peers.refresh_peer_on_gossip(self.node_id) {
            debug!(
                self.logger,
                "received gossip from node that is not in the peer map",
            );
        }
        self.global_state.topology.update(nodes);
    }
}

impl Sink for BlockAnnouncementProcessor {
    type SinkItem = Header;
    type SinkError = ();

    fn start_send(&mut self, header: Header) -> StartSend<Header, ()> {
        let polled = self
            .mbox
            .start_send(BlockMsg::AnnouncedBlock(header, self.node_id))
            .map_err(|e| {
                error!(
                    self.logger,
                    "failed to send block announcement to the block task";
                    "reason" => %e,
                );
            })?;
        match polled {
            AsyncSink::Ready => {
                self.global_state.peers.refresh_peer_on_block(self.node_id);
                Ok(AsyncSink::Ready)
            }
            AsyncSink::NotReady(BlockMsg::AnnouncedBlock(header, _)) => {
                Ok(AsyncSink::NotReady(header))
            }
            AsyncSink::NotReady(_) => unreachable!(),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), ()> {
        self.mbox.poll_complete().map_err(|e| {
            error!(
                self.logger,
                "communication channel to the block task failed";
                "reason" => %e,
            );
        })
    }

    fn close(&mut self) -> Poll<(), ()> {
        self.mbox.close().map_err(|e| {
            warn!(
                self.logger,
                "failed to close communication channel to the block task";
                "reason" => %e,
            );
        })
    }
}

impl Sink for FragmentProcessor {
    type SinkItem = Vec<Fragment>;
    type SinkError = ();

    fn start_send(&mut self, fragments: Vec<Fragment>) -> StartSend<Self::SinkItem, ()> {
        let polled = self
            .mbox
            .start_send(TransactionMsg::SendTransaction(
                FragmentOrigin::Network,
                fragments,
            ))
            .map_err(|e| {
                error!(
                    self.logger,
                    "failed to send fragments to the fragment task";
                    "reason" => %e,
                );
            })?;
        match polled {
            AsyncSink::Ready => {
                self.global_state
                    .peers
                    .refresh_peer_on_fragment(self.node_id);
                Ok(AsyncSink::Ready)
            }
            AsyncSink::NotReady(TransactionMsg::SendTransaction(_, fragments)) => {
                Ok(AsyncSink::NotReady(fragments))
            }
            AsyncSink::NotReady(_) => unreachable!(),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), ()> {
        self.mbox.poll_complete().map_err(|e| {
            error!(
                self.logger,
                "communication channel to the fragment task failed";
                "reason" => %e,
            );
        })
    }

    fn close(&mut self) -> Poll<(), ()> {
        self.mbox.close().map_err(|e| {
            warn!(
                self.logger,
                "failed to close communication channel to the fragment task";
                "reason" => %e,
            );
        })
    }
}
