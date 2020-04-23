use super::{
    buffer_sizes,
    convert::Decode,
    p2p::{Address, Gossip},
    GlobalStateR,
};
use crate::{
    blockcfg::{Fragment, Header},
    intercom::{BlockMsg, TransactionMsg},
    settings::start::network::Configuration,
    utils::async_msg::{self, MessageBox},
};
use chain_network::data as net_data;
use chain_network::error::{Code, Error};
use jormungandr_lib::interfaces::FragmentOrigin;

use futures03::future::BoxFuture;
use futures03::prelude::*;
use futures03::ready;
use slog::Logger;

use std::pin::Pin;
use std::task::{Context, Poll};

fn filter_gossip_node(node: &Gossip, config: &Configuration) -> bool {
    if config.allow_private_addresses {
        node.has_valid_address()
    } else {
        node.is_global()
    }
}

fn handle_mbox_error(err: async_msg::SendError, logger: Logger) -> Error {
    error!(
        logger,
        "failed to send block announcement to the block task";
        "reason" => %err,
    );
    Error::new(Code::Internal, err)
}

pub async fn process_block_announcements<S>(
    stream: S,
    mbox: MessageBox<BlockMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    logger: Logger,
) where
    S: TryStream<Ok = net_data::Header, Error = Error>,
{
    let res = stream
        .try_for_each(|raw_header| async move {
            let header = Header::from_slice(raw_header.as_bytes())
                .map_err(|e| Error::new(Code::InvalidArgument, e))?;
            mbox.send(BlockMsg::AnnouncedBlock(header, node_id))
                .await
                .map_err(|e| handle_mbox_error(e, logger))?;
            if !global_state.peers.refresh_peer_on_block(node_id).await {
                debug!(
                    logger,
                    "received block from node that is not in the peer map",
                );
            }
            Ok(())
        })
        .await;
    if let Err(e) = res {
        debug!(
            logger,
            "inbound subscription stream failed";
            "error" => ?e,
        );
    }
}

pub async fn process_gossip<S>(
    stream: S,
    node_id: Address,
    global_state: GlobalStateR,
    logger: Logger,
) where
    S: TryStream<Ok = net_data::Gossip, Error = Error>,
{
    let processor = GossipProcessor::new(node_id, global_state, logger);
    let res = stream
        .try_for_each(|item| async move {
            processor.start_processing_item(item)?;
            future::poll_fn(|cx| processor.poll_ready(cx)).await;
            Ok(())
        })
        .await;
    if let Err(e) = res {
        debug!(
            logger,
            "inbound subscription stream failed";
            "error" => ?e,
        );
    }
}

pub async fn process_fragments<S>(
    stream: S,
    mbox: MessageBox<TransactionMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    logger: Logger,
) where
    S: TryStream<Ok = net_data::Fragment, Error = Error>,
{
    let stream = stream.and_then(|fragment| async { fragment.decode() });
    let sink = FragmentProcessor::new(mbox, node_id, global_state, logger);
    stream.into_stream().forward(sink).await.map_err(|e| {
        debug!(logger, "processing of inbound subscription stream failed"; "error" => ?e);
    });
}

// TODO: replace with a suitable stream combinator once implemented:
// https://github.com/rust-lang/futures-rs/issues/1919
#[must_use = "sinks do nothing unless polled"]
struct FragmentProcessor {
    mbox: MessageBox<TransactionMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    logger: Logger,
    buffered_fragments: Vec<Fragment>,
    refresh_stat_future: Option<BoxFuture<'static, ()>>,
}

impl FragmentProcessor {
    fn new(
        mbox: MessageBox<TransactionMsg>,
        node_id: Address,
        global_state: GlobalStateR,
        logger: Logger,
    ) -> Self {
        FragmentProcessor {
            mbox,
            node_id,
            global_state,
            logger,
            buffered_fragments: Vec::new(),
            refresh_stat_future: None,
        }
    }

    fn refresh_stat(&mut self) {
        let refresh_logger = self.logger.clone();
        let state = self.global_state.clone();
        let node_id = self.node_id;
        let fut = Box::pin(async move {
            let refreshed = state.peers.refresh_peer_on_fragment(node_id).await;
            if !refreshed {
                debug!(
                    refresh_logger,
                    "received fragment from node that is not in the peer map",
                );
            }
        });
        // It's OK to overwrite a pending future because only the latest
        // timestamp matters.
        self.refresh_stat_future = Some(fut);
    }
}

pub struct GossipProcessor {
    node_id: Address,
    global_state: GlobalStateR,
    logger: Logger,
    pending_processing: Option<BoxFuture<'static, ()>>,
}

impl GossipProcessor {
    pub fn new(node_id: Address, global_state: GlobalStateR, logger: Logger) -> Self {
        GossipProcessor {
            node_id,
            global_state,
            logger,
            pending_processing: None,
        }
    }

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(fut) = &mut self.pending_processing {
            ready!(Pin::new(fut).poll(cx));
            self.pending_processing = None;
        }
        Poll::Ready(())
    }

    fn start_processing_item(&mut self, gossip: net_data::Gossip) -> Result<(), Error> {
        let nodes = gossip.nodes.decode()?;
        let (nodes, filtered_out): (Vec<_>, Vec<_>) = nodes.into_iter().partition(|node| {
            filter_gossip_node(node, &self.global_state.config) || node.address().is_none()
        });
        if filtered_out.len() > 0 {
            debug!(self.logger, "nodes dropped from gossip: {:?}", filtered_out);
        }
        let node_id = self.node_id;
        let state1 = self.global_state.clone();
        let state2 = self.global_state.clone();
        let logger = self.logger.clone();
        let fut = future::join(
            async move {
                let refreshed = state1.peers.refresh_peer_on_gossip(node_id).await;
                if !refreshed {
                    debug!(
                        logger,
                        "received gossip from node that is not in the peer map",
                    );
                }
            },
            async move {
                state2.topology.accept_gossips(node_id, nodes.into()).await;
            },
        )
        .map(|_| ());
        self.pending_processing = Some(Box::pin(fut));
        Ok(())
    }
}

impl Sink<Fragment> for FragmentProcessor {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        if self.buffered_fragments.len() >= buffer_sizes::inbound::FRAGMENTS {
            ready!(self.poll_send_fragments(cx));
            debug_assert!(self.buffered_fragments.is_empty());
        }
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, fragment: Fragment) -> Result<(), Self::Error> {
        assert!(
            self.buffered_fragments.len() < buffer_sizes::inbound::FRAGMENTS,
            "should call `poll_ready` which returns `Poll::Ready(Ok(()))` before `start_send`",
        );
        self.buffered_fragments.push(fragment);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        loop {
            if self.buffered_fragments.is_empty() {
                self.poll_complete_refresh_stat(cx);
                return Pin::new(&mut self.mbox).poll_flush(cx).map_err(|e| {
                    error!(
                        self.logger,
                        "communication channel to the fragment task failed";
                        "reason" => %e,
                    );
                    Error::new(Code::Internal, e)
                });
            } else {
                ready!(self.poll_send_fragments(cx));
            }
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        loop {
            if self.buffered_fragments.is_empty() {
                ready!(self.poll_complete_refresh_stat(cx));
                return Pin::new(&mut self.mbox).poll_close(cx).map_err(|e| {
                    warn!(
                        self.logger,
                        "failed to close communication channel to the fragment task";
                        "reason" => %e,
                    );
                    Error::new(Code::Internal, e)
                });
            } else {
                ready!(self.poll_send_fragments(cx));
            }
        }
    }
}

impl FragmentProcessor {
    fn poll_send_fragments(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        ready!(self.mbox.poll_ready(cx));
        let fragments = self.buffered_fragments.split_off(0);
        self.mbox
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
                Error::new(Code::Internal, e)
            })?;
        self.refresh_stat();
        Poll::Ready(Ok(()))
    }

    fn poll_complete_refresh_stat(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(fut) = &mut self.refresh_stat_future {
            ready!(Pin::new(fut).poll(cx));
            self.refresh_stat_future = None;
        }
        Poll::Ready(())
    }
}
