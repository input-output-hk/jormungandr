mod connect;

pub use self::connect::{connect, ConnectError, ConnectFuture, ConnectHandle};
use super::{
    buffer_sizes,
    convert::{Decode, Encode},
    grpc::{
        self,
        client::{BlockSubscription, FragmentSubscription, GossipSubscription},
    },
    p2p::comm::{OutboundSubscription, PeerComms},
    subscription::{BlockAnnouncementProcessor, Direction, FragmentProcessor, GossipProcessor},
    Channels, GlobalStateR,
};
use crate::{
    intercom::{self, BlockMsg, ClientMsg},
    topology::NodeId,
    utils::async_msg::MessageBox,
};
use chain_network::{
    data as net_data,
    data::block::{BlockEvent, BlockIds, ChainPullRequest},
};
use futures::{prelude::*, ready};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tracing::{instrument, Span};
use tracing_futures::Instrument;

#[must_use = "Client must be polled"]
pub struct Client {
    inner: grpc::Client,
    global_state: GlobalStateR,
    inbound: InboundSubscriptions,
    block_solicitations: OutboundSubscription<BlockIds>,
    chain_pulls: OutboundSubscription<ChainPullRequest>,
    block_sink: BlockAnnouncementProcessor,
    fragment_sink: FragmentProcessor,
    gossip_sink: GossipProcessor,
    client_box: MessageBox<ClientMsg>,
    incoming_block_announcement: Option<net_data::Header>,
    incoming_solicitation: Option<ClientMsg>,
    shutting_down: bool,
    span: Span,
}

struct ClientBuilder {
    pub span: Span,
    pub channels: Channels,
}

impl Client {
    pub fn span(&self) -> &Span {
        &self.span
    }
}

impl Client {
    fn new(
        inner: grpc::Client,
        builder: ClientBuilder,
        global_state: GlobalStateR,
        inbound: InboundSubscriptions,
        comms: &mut PeerComms,
    ) -> Self {
        let parent_span = builder.span;

        let block_sink = BlockAnnouncementProcessor::new(
            builder.channels.block_box,
            inbound.peer_id,
            global_state.clone(),
        );
        let fragment_sink = FragmentProcessor::new(
            builder.channels.transaction_box,
            inbound.peer_id,
            global_state.clone(),
        );
        let gossip_sink = GossipProcessor::new(
            builder.channels.topology_box,
            inbound.peer_id,
            global_state.clone(),
            Direction::Client,
        );

        Client {
            inner,
            global_state,
            inbound,
            block_solicitations: comms.subscribe_to_block_solicitations(),
            chain_pulls: comms.subscribe_to_chain_pulls(),
            block_sink,
            fragment_sink,
            gossip_sink,
            client_box: builder.channels.client_box,
            incoming_block_announcement: None,
            incoming_solicitation: None,
            shutting_down: false,
            span: parent_span,
        }
    }
}

struct InboundSubscriptions {
    pub peer_id: NodeId,
    pub block_events: BlockSubscription,
    pub fragments: FragmentSubscription,
    pub gossip: GossipSubscription,
}

#[derive(Copy, Clone)]
enum ProcessingOutcome {
    Continue,
    Disconnect,
}

struct Progress(pub Poll<ProcessingOutcome>);

impl Progress {
    fn begin(async_outcome: Poll<Result<ProcessingOutcome, ()>>) -> Self {
        use self::ProcessingOutcome::*;

        Progress(async_outcome.map(|res| res.unwrap_or(Disconnect)))
    }

    fn and_proceed_with<F>(&mut self, poll_fn: F)
    where
        F: FnOnce() -> Poll<Result<ProcessingOutcome, ()>>,
    {
        use self::ProcessingOutcome::*;
        use Poll::*;

        let async_outcome = match self.0 {
            Pending | Ready(Continue) => poll_fn(),
            Ready(Disconnect) => return,
        };

        if let Ready(outcome) = async_outcome {
            match outcome {
                Ok(outcome) => {
                    self.0 = Ready(outcome);
                }
                Err(()) => {
                    self.0 = Ready(Disconnect);
                }
            }
        }
    }
}

impl Client {
    #[instrument(skip_all, level = "debug")]
    fn process_block_event(&mut self, cx: &mut Context<'_>) -> Poll<Result<ProcessingOutcome, ()>> {
        use self::ProcessingOutcome::*;
        // Drive sending of a message to block task to clear the buffered
        // announcement before polling more events from the block subscription
        // stream.
        let mut block_sink = Pin::new(&mut self.block_sink);
        ready!(block_sink.as_mut().poll_ready(cx))
            .map_err(|e| tracing::debug!(reason = %e, "failed getting block sink"))?;
        if let Some(header) = self.incoming_block_announcement.take() {
            block_sink.start_send(header).map_err(|_| ())?;
        } else {
            match block_sink.as_mut().poll_flush(cx) {
                Poll::Pending => {
                    // Ignoring possible Pending return here: due to the following
                    // ready!() invocations, this function cannot return Continue
                    // while no progress has been made.
                    Ok(())
                }
                Poll::Ready(Ok(())) => Ok(()),
                Poll::Ready(Err(_)) => Err(()),
            }?;
        }

        // Drive sending of a message to the client request task to clear
        // the buffered solicitation before polling more events from the
        // block subscription stream.
        let mut client_box = Pin::new(&mut self.client_box);
        ready!(client_box.as_mut().poll_ready(cx)).map_err(|e| {
            tracing::error!(
                reason = %e,
                "processing of incoming client requests failed"
            );
        })?;
        if let Some(msg) = self.incoming_solicitation.take() {
            client_box.start_send(msg).map_err(|e| {
                tracing::error!(
                    reason = %e,
                    "failed to send client request for processing"
                );
            })?;
        } else {
            match client_box.as_mut().poll_flush(cx) {
                Poll::Pending => {
                    // Ignoring possible Pending return here: due to the following
                    // ready!() invocation, this function cannot return Continue
                    // while no progress has been made.
                    Ok(())
                }
                Poll::Ready(Ok(())) => Ok(()),
                Poll::Ready(Err(e)) => {
                    tracing::error!(
                        reason = %e,
                        "processing of incoming client requests failed"
                    );
                    Err(())
                }
            }?;
        }

        let block_events = Pin::new(&mut self.inbound.block_events);
        let maybe_event = ready!(block_events.poll_next(cx));
        let event = match maybe_event {
            Some(Ok(event)) => event,
            None => {
                tracing::debug!("block event subscription ended by the peer");
                return Ok(Disconnect).into();
            }
            Some(Err(e)) => {
                tracing::debug!(
                    error = ?e,
                    "block subscription stream failure"
                );
                return Err(()).into();
            }
        };
        match event {
            BlockEvent::Announce(header) => {
                debug_assert!(self.incoming_block_announcement.is_none());
                self.incoming_block_announcement = Some(header);
            }
            BlockEvent::Solicit(block_ids) => {
                self.upload_blocks(block_ids)?;
            }
            BlockEvent::Missing(req) => {
                self.push_missing_headers(req)?;
            }
        }
        Ok(Continue).into()
    }

    #[instrument(skip_all, level = "debug")]
    fn upload_blocks(&mut self, block_ids: BlockIds) -> Result<(), ()> {
        if block_ids.is_empty() {
            tracing::info!("peer has sent an empty block solicitation");
            return Err(());
        }
        let block_ids = block_ids.decode().map_err(|e| {
            tracing::info!(
                reason = %e,
                "failed to decode block IDs from solicitation request"
            );
        })?;
        tracing::info!(
            "peer requests {} blocks starting from {}",
            block_ids.len(),
            block_ids[0]
        );
        let (reply_handle, future) = intercom::stream_reply(buffer_sizes::outbound::BLOCKS);
        debug_assert!(self.incoming_solicitation.is_none());
        self.incoming_solicitation = Some(ClientMsg::GetBlocks(block_ids, reply_handle));
        let mut client = self.inner.clone();
        self.global_state.spawn(
            async move {
                let stream = match future.await {
                    Ok(stream) => stream.upload().map(|item| item.encode()),
                    Err(e) => {
                        tracing::info!(
                            reason = %e,
                            "cannot serve peer's solicitation"
                        );
                        return;
                    }
                };
                match client.upload_blocks(stream).await {
                    Ok(()) => {
                        tracing::debug!("finished uploading blocks");
                    }
                    Err(e) => {
                        tracing::info!(
                            error = ?e,
                            "UploadBlocks request failed"
                        );
                    }
                }
            }
            .in_current_span(),
        );
        Ok(())
    }

    #[instrument(skip_all, level = "debug")]
    fn push_missing_headers(&mut self, req: ChainPullRequest) -> Result<(), ()> {
        let from = req.from.decode().map_err(|e| {
            tracing::info!(
                reason = %e,
                "failed to decode checkpoint block IDs from header pull request"
            );
        })?;
        let to = req.to.decode().map_err(|e| {
            tracing::info!(
                reason = %e,
                "failed to decode tip block ID from header pull request"
            );
        })?;
        tracing::debug!(
            checkpoints = ?from,
            to = ?to,
            "peer requests missing part of the chain"
        );
        let (reply_handle, future) = intercom::stream_reply(buffer_sizes::outbound::HEADERS);
        debug_assert!(self.incoming_solicitation.is_none());
        self.incoming_solicitation = Some(ClientMsg::PullHeaders(from, to, reply_handle));
        let mut client = self.inner.clone();
        self.global_state.spawn(
            async move {
                let stream = match future.await {
                    Ok(stream) => stream.upload().map(|item| item.encode()),
                    Err(e) => {
                        tracing::info!(
                            reason = %e,
                            "cannot serve peer's solicitation"
                        );
                        return;
                    }
                };
                match client.push_headers(stream).await {
                    Ok(()) => {
                        tracing::debug!("finished pushing headers");
                    }
                    Err(e) => {
                        tracing::info!(
                            error = ?e,
                            "PushHeaders request failed"
                        );
                    }
                }
            }
            .in_current_span(),
        );
        Ok(())
    }

    #[instrument(skip_all, level = "debug")]
    fn pull_headers(&mut self, req: ChainPullRequest) {
        let mut block_box = self.block_sink.message_box();

        let (handle, sink, _) = intercom::stream_request(buffer_sizes::inbound::HEADERS);
        // TODO: make sure that back pressure on the number of requests
        // in flight prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state.spawn(
            async move {
                let res = block_box.send(BlockMsg::ChainHeaders(handle)).await;
                if let Err(e) = res {
                    tracing::error!(
                        reason = %e,
                        "failed to enqueue request for processing"
                    );
                }
            }
            .in_current_span(),
        );
        let mut client = self.inner.clone();
        self.global_state.spawn(
            async move {
                match client.pull_headers(req.from, req.to).await {
                    Err(e) => {
                        tracing::info!(
                            reason = %e,
                            "request failed"
                        );
                    }
                    Ok(stream) => {
                        let stream = stream.and_then(|item| async { item.decode() });
                        let res = stream.forward(sink.sink_err_into()).await;
                        if let Err(e) = res {
                            tracing::info!(
                                reason = %e,
                                "response stream failed"
                            );
                        }
                    }
                }
            }
            .in_current_span(),
        );
    }

    #[instrument(skip_all, level = "debug")]
    fn solicit_blocks(&mut self, block_ids: BlockIds) {
        let mut block_box = self.block_sink.message_box();
        let (handle, sink, _) = intercom::stream_request(buffer_sizes::inbound::BLOCKS);
        // TODO: make sure that back pressure on the number of requests
        // in flight prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state.spawn(
            async move {
                let res = block_box.send(BlockMsg::NetworkBlocks(handle)).await;
                if let Err(e) = res {
                    tracing::error!(
                        reason = %e,
                        "failed to enqueue request for processing"
                    );
                }
            }
            .in_current_span(),
        );
        let mut client = self.inner.clone();
        self.global_state.spawn(
            async move {
                match client.get_blocks(block_ids).await {
                    Err(e) => {
                        tracing::info!(
                            reason = %e,
                            "request failed"
                        );
                    }
                    Ok(stream) => {
                        let stream = stream.and_then(|item| async { item.decode() });
                        let res = stream.forward(sink.sink_err_into()).await;
                        if let Err(e) = res {
                            tracing::info!(
                                reason = %e,
                                "response stream failed"
                            );
                        }
                    }
                }
            }
            .in_current_span(),
        );
    }

    #[instrument(skip_all, level = "debug", fields(direction = "in"))]
    fn process_fragments(&mut self, cx: &mut Context<'_>) -> Poll<Result<ProcessingOutcome, ()>> {
        use self::ProcessingOutcome::*;
        let mut fragment_sink = Pin::new(&mut self.fragment_sink);
        ready!(fragment_sink.as_mut().poll_ready(cx)).map_err(|_| ())?;

        match Pin::new(&mut self.inbound.fragments).poll_next(cx) {
            Poll::Pending => {
                if let Poll::Ready(Err(_)) = fragment_sink.as_mut().poll_flush(cx) {
                    return Err(()).into();
                }
                Poll::Pending
            }
            Poll::Ready(Some(Ok(fragment))) => {
                fragment_sink
                    .as_mut()
                    .start_send(fragment)
                    .map_err(|_| ())?;
                Ok(Continue).into()
            }
            Poll::Ready(None) => {
                tracing::debug!("fragment subscription ended by the peer");
                Ok(Disconnect).into()
            }
            Poll::Ready(Some(Err(e))) => {
                tracing::debug!(
                    error = ?e,
                    "fragment subscription stream failure"
                );
                Err(()).into()
            }
        }
    }

    #[instrument(skip_all, level = "debug", fields(direction = "in"))]
    fn process_gossip(&mut self, cx: &mut Context<'_>) -> Poll<Result<ProcessingOutcome, ()>> {
        use self::ProcessingOutcome::*;
        let mut gossip_sink = Pin::new(&mut self.gossip_sink);
        ready!(gossip_sink.as_mut().poll_ready(cx)).map_err(|_| ())?;

        match Pin::new(&mut self.inbound.gossip).poll_next(cx) {
            Poll::Pending => {
                if let Poll::Ready(Err(_)) = gossip_sink.as_mut().poll_flush(cx) {
                    return Err(()).into();
                }
                Poll::Pending
            }
            Poll::Ready(Some(Ok(gossip))) => {
                tracing::debug!("client");
                gossip_sink.as_mut().start_send(gossip).map_err(|_| ())?;
                Ok(Continue).into()
            }
            Poll::Ready(None) => {
                tracing::debug!("gossip subscription ended by the peer");
                Ok(Disconnect).into()
            }
            Poll::Ready(Some(Err(e))) => {
                tracing::debug!(
                    error = ?e,
                    "gossip subscription stream failure"
                );
                Err(()).into()
            }
        }
    }

    fn poll_shut_down(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let span = self.span().clone();
        let _enter = span.enter();
        ready!(Pin::new(&mut self.block_sink).poll_close(cx)).unwrap_or(());
        ready!(Pin::new(&mut self.fragment_sink).poll_close(cx)).unwrap_or(());
        ready!(Pin::new(&mut self.gossip_sink).poll_close(cx)).unwrap_or(());
        ready!(Pin::new(&mut self.client_box).poll_close(cx)).unwrap_or_else(|e| {
            tracing::warn!(
                reason = %e,
                "failed to close communication channel to the client task"
            );
        });
        Poll::Ready(())
    }
}

impl Future for Client {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        use self::ProcessingOutcome::*;

        if self.shutting_down {
            return self.poll_shut_down(cx);
        }

        loop {
            let mut progress = Progress::begin(self.process_block_event(cx));

            progress.and_proceed_with(|| self.process_fragments(cx));
            progress.and_proceed_with(|| self.process_gossip(cx));

            // Block solicitations and chain pulls are special:
            // they are handled with client requests on the client side,
            // but on the server side, they are fed into the block event stream.
            progress.and_proceed_with(|| {
                Pin::new(&mut self.block_solicitations)
                    .poll_next(cx)
                    .map(|maybe_item| match maybe_item {
                        Some(block_ids) => {
                            self.solicit_blocks(block_ids);
                            Ok(Continue)
                        }
                        None => {
                            tracing::debug!("outbound block solicitation stream closed");
                            Ok(Disconnect)
                        }
                    })
            });
            progress.and_proceed_with(|| {
                Pin::new(&mut self.chain_pulls)
                    .poll_next(cx)
                    .map(|maybe_item| match maybe_item {
                        Some(req) => {
                            self.pull_headers(req);
                            Ok(Continue)
                        }
                        None => {
                            tracing::debug!("outbound header pull stream closed");
                            Ok(Disconnect)
                        }
                    })
            });

            match progress {
                Progress(Poll::Pending) => return Poll::Pending,
                Progress(Poll::Ready(Continue)) => continue,
                Progress(Poll::Ready(Disconnect)) => {
                    tracing::info!("disconnecting client");
                    return ().into();
                }
            }
        }
    }
}
