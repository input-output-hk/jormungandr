mod connect;

use super::{
    buffer_sizes,
    convert::{Decode, Encode},
    grpc::{
        self,
        client::{BlockSubscription, FragmentSubscription, GossipSubscription},
    },
    p2p::{
        comm::{OutboundSubscription, PeerComms},
        Address,
    },
    subscription::{BlockAnnouncementProcessor, FragmentProcessor, GossipProcessor},
    Channels, GlobalStateR,
};
use crate::{
    blockcfg::{Block, Fragment, Header, HeaderHash},
    intercom::{self, BlockMsg, ClientMsg},
    utils::async_msg::MessageBox,
};
use chain_network::data as net_data;
use chain_network::data::block::{BlockEvent, BlockIds, ChainPullRequest};
use chain_network::error as net_error;

use futures03::prelude::*;
use futures03::ready;
use slog::Logger;

use std::pin::Pin;
use std::task::{Context, Poll};

pub use self::connect::{connect, ConnectError, ConnectFuture, ConnectHandle};

#[must_use = "Client must be polled"]
pub struct Client {
    inner: grpc::Client,
    logger: Logger,
    global_state: GlobalStateR,
    inbound: InboundSubscriptions,
    block_solicitations: OutboundSubscription<BlockIds>,
    chain_pulls: OutboundSubscription<ChainPullRequest>,
    block_sink: BlockAnnouncementProcessor,
    fragment_sink: FragmentProcessor,
    gossip_processor: GossipProcessor,
    client_box: MessageBox<ClientMsg>,
    incoming_block_announcement: Option<net_data::Header>,
    incoming_solicitation: Option<ClientMsg>,
    incoming_fragment: Option<net_data::Fragment>,
}

struct ClientBuilder {
    pub logger: Logger,
    pub channels: Channels,
}

impl Client {
    pub fn remote_node_id(&self) -> Address {
        self.inbound.node_id
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
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
        let remote_node_id = inbound.node_id;
        let logger = builder
            .logger
            .new(o!("node_id" => remote_node_id.to_string()));

        let block_sink = BlockAnnouncementProcessor::new(
            builder.channels.block_box,
            remote_node_id,
            global_state.clone(),
            logger.new(o!("stream" => "block_events", "direction" => "in")),
        );
        let fragment_sink = FragmentProcessor::new(
            builder.channels.transaction_box,
            remote_node_id,
            global_state.clone(),
            logger.new(o!("stream" => "fragments", "direction" => "in")),
        );
        let gossip_processor = GossipProcessor::new(
            remote_node_id,
            global_state.clone(),
            logger.new(o!("stream" => "gossip", "direction" => "in")),
        );

        Client {
            inner,
            logger,
            global_state,
            inbound,
            block_solicitations: comms.subscribe_to_block_solicitations(),
            chain_pulls: comms.subscribe_to_chain_pulls(),
            block_sink,
            fragment_sink,
            gossip_processor,
            client_box: builder.channels.client_box,
            incoming_block_announcement: None,
            incoming_solicitation: None,
            incoming_fragment: None,
        }
    }
}

struct InboundSubscriptions {
    pub node_id: Address,
    pub block_events: BlockSubscription,
    pub fragments: FragmentSubscription,
    pub gossip: GossipSubscription,
}

#[derive(Copy, Clone)]
enum ProcessingOutcome {
    Continue,
    Disconnect,
}

struct Progress(pub Option<ProcessingOutcome>);

impl Progress {
    fn update(&mut self, async_outcome: Poll<ProcessingOutcome>) {
        use self::ProcessingOutcome::*;
        if let Poll::Ready(outcome) = async_outcome {
            match (self.0, outcome) {
                (None, outcome) | (Some(Continue), outcome) => {
                    self.0 = Some(outcome);
                }
                (Some(Disconnect), _) => {}
            }
        }
    }
}

impl Client {
    fn process_block_event(&mut self, cx: &mut Context<'_>) -> Poll<Result<ProcessingOutcome, ()>> {
        use self::ProcessingOutcome::*;

        // Drive sending of a message to block task to clear the buffered
        // announcement before polling more events from the block subscription
        // stream.
        let block_sink = Pin::new(&mut self.block_sink);
        ready!(block_sink.poll_ready(cx));
        if let Some(header) = self.incoming_block_announcement.take() {
            block_sink.start_send(header).map_err(|e| {
                error!(
                    self.logger,
                    "failed to send announced block header for processing";
                    "reason" => %e,
                );
            })?;
        } else {
            // Ignoring possible Pending return here: due to the following
            // ready!() invocations, this function cannot return Continue
            // while no progress has been made.
            block_sink.poll_flush(cx).map_err(|e| {
                error!(
                    self.logger,
                    "processing of incoming block messages failed";
                    "reason" => %e,
                );
            })?;
        }

        // Drive sending of a message to the client request task to clear
        // the buffered solicitation before polling more events from the
        // block subscription stream.
        let client_box = Pin::new(&mut self.client_box);
        ready!(client_box.poll_ready(cx));
        if let Some(msg) = self.incoming_solicitation.take() {
            client_box.start_send(msg).map_err(|e| {
                error!(
                    self.logger,
                    "failed to send client request for processing";
                    "reason" => %e,
                );
            })?;
        } else {
            // Ignoring possible Pending return here: due to the following
            // ready!() invocation, this function cannot return Continue
            // while no progress has been made.
            client_box.poll_flush(cx).map_err(|e| {
                error!(
                    self.logger,
                    "processing of incoming client requests failed";
                    "reason" => %e,
                );
            })?;
        }

        let block_events = Pin::new(&mut self.inbound.block_events);
        let maybe_event = ready!(block_events.poll_next(cx));
        let event = match maybe_event {
            Some(Ok(event)) => event,
            None => {
                debug!(self.logger, "block event subscription ended by the peer");
                return Ok(Disconnect).into();
            }
            Some(Err(e)) => {
                debug!(
                    self.logger,
                    "block subscription stream failure";
                    "error" => ?e,
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

    fn upload_blocks(&mut self, block_ids: BlockIds) -> Result<(), ()> {
        debug!(self.logger, "peer requests {} blocks", block_ids.len());
        let block_ids = block_ids.decode().map_err(|e| {
            info!(
                self.logger,
                "failed to decode block IDs from solicitation request";
                "reason" => %e,
            );
        })?;
        let (reply_handle, stream) = intercom::upload_stream_reply(
            buffer_sizes::outbound::BLOCKS,
            self.logger.new(o!("solicitation" => "UploadBlocks")),
        );
        debug_assert!(self.incoming_solicitation.is_none());
        self.incoming_solicitation = Some(ClientMsg::GetBlocks(block_ids, reply_handle));
        let stream = stream.map(|res| res.encode());
        let client = self.inner.clone();
        let logger = self.logger.clone();
        self.global_state.spawn(async move {
            match client.upload_blocks(stream).await {
                Ok(()) => {
                    debug!(logger, "finished uploading blocks");
                }
                Err(e) => {
                    info!(
                        logger,
                        "UploadBlocks request failed";
                        "error" => ?e,
                    );
                }
            }
        });
        Ok(())
    }

    fn push_missing_headers(&mut self, req: ChainPullRequest) -> Result<(), ()> {
        let from = req.from.decode().map_err(|e| {
            info!(
                self.logger,
                "failed to decode checkpoint block IDs from header pull request";
                "reason" => %e,
            );
        })?;
        let to = req.to.decode().map_err(|e| {
            info!(
                self.logger,
                "failed to decode tip block ID from header pull request";
                "reason" => %e,
            );
        })?;
        debug!(
            self.logger,
            "peer requests missing part of the chain";
            "checkpoints" => ?from,
            "to" => ?to,
        );
        let (reply_handle, stream) = intercom::upload_stream_reply(
            buffer_sizes::outbound::HEADERS,
            self.logger.new(o!("solicitation" => "PushHeaders")),
        );
        debug_assert!(self.incoming_solicitation.is_none());
        self.incoming_solicitation = Some(ClientMsg::GetHeadersRange(from, to, reply_handle));
        let stream = stream.map(|res| res.encode());
        let client = self.inner.clone();
        let logger = self.logger.clone();
        self.global_state.spawn(async move {
            match client.push_headers(stream).await {
                Ok(()) => {
                    debug!(logger, "finished pushing headers");
                }
                Err(e) => {
                    info!(
                        logger,
                        "PushHeaders request failed";
                        "error" => ?e,
                    );
                }
            }
        });
        Ok(())
    }

    fn pull_headers(&mut self, req: ChainPullRequest) {
        let block_box = self.block_sink.message_box();
        let logger = self.logger.new(o!("request" => "PullHeaders"));
        let logger1 = logger.clone();
        let (handle, sink) =
            intercom::stream_request(buffer_sizes::inbound::HEADERS, logger.clone());
        // TODO: make sure that back pressure on the number of requests
        // in flight prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state.spawn(async move {
            let res = block_box.send(BlockMsg::ChainHeaders(handle)).await;
            if let Err(e) = res {
                error!(
                    logger,
                    "failed to enqueue request for processing";
                    "reason" => %e,
                );
            }
        });
        let client = self.inner.clone();
        self.global_state.spawn(async move {
            match client.pull_headers(req.from, req.to).await {
                Err(e) => {
                    info!(
                        logger1,
                        "request failed";
                        "reason" => %e,
                    );
                }
                Ok(stream) => {
                    let stream = stream.and_then(|item| async { item.decode() });
                    let res = stream.forward(sink.sink_err_into()).await;
                    if let Err(e) = res {
                        info!(
                            logger1,
                            "response stream failed";
                            "reason" => %e,
                        );
                    }
                }
            }
        });
    }

    fn solicit_blocks(&mut self, block_ids: BlockIds) {
        let block_box = self.block_sink.message_box();
        let logger = self.logger.new(o!("request" => "GetBlocks"));
        let req_err_logger = logger.clone();
        let res_logger = logger.clone();
        let (handle, sink) =
            intercom::stream_request(buffer_sizes::inbound::BLOCKS, logger.clone());
        // TODO: make sure that back pressure on the number of requests
        // in flight prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state.spawn(async move {
            let res = block_box.send(BlockMsg::NetworkBlocks(handle)).await;
            if let Err(e) = res {
                error!(
                    logger,
                    "failed to enqueue request for processing";
                    "reason" => %e,
                );
            }
        });
        let client = self.inner.clone();
        self.global_state.spawn(async move {
            match client.get_blocks(block_ids).await {
                Err(e) => {
                    info!(
                        req_err_logger,
                        "request failed";
                        "reason" => %e,
                    );
                }
                Ok(stream) => {
                    let stream = stream.and_then(|item| async { item.decode() });
                    let res = stream.forward(sink.sink_err_into()).await;
                    if let Err(e) = res {
                        info!(
                            res_logger,
                            "response stream failed";
                            "reason" => %e,
                        );
                    }
                }
            }
        });
    }

    fn process_fragments(&mut self, cx: &mut Context<'_>) -> Poll<Result<ProcessingOutcome, ()>> {
        use self::ProcessingOutcome::*;

        // Drive sending of a message to fragment task to completion
        // before polling more events from the fragment subscription
        // stream.
        let fragment_box = &mut self.channels.transaction_box;
        ready!(fragment_box.poll_ready(cx));
        if let Some(fragment) = self.incoming_fragment.take() {
            fragment_box.start_send(fragment).map_err(|e| {
                error!(
                    self.logger,
                    "failed to send fragment for processing";
                    "reason" => %e,
                );
            })?;
        } else {
            // Ignoring possible Pending return here: due to the following
            // try_ready!() invocation, this function cannot return Continue
            // while no progress has been made.
            Pin::new(fragment_box).poll_flush(cx).map_err(|e| {
                error!(
                    self.logger,
                    "processing of incoming fragments failed";
                    "reason" => %e,
                );
            })?;
        }

        let stream = Pin::new(&mut self.inbound.fragments);
        let maybe_fragment = ready!(stream.poll_next(cx));
        match maybe_fragment {
            Some(Ok(fragment)) => {
                debug_assert!(self.incoming_fragment.is_none());
                self.incoming_fragment = Some(fragment);
                Ok(Continue).into()
            }
            None => {
                debug!(self.logger, "fragment subscription ended by the peer");
                Ok(Disconnect).into()
            }
            Some(Err(e)) => {
                debug!(
                    self.logger,
                    "fragment stream failure";
                    "error" => %e,
                );
                Err(()).into()
            }
        }
    }

    fn process_gossip(&mut self, cx: &mut Context<'_>) -> Poll<Result<ProcessingOutcome, ()>> {
        use self::ProcessingOutcome::*;

        ready!(self.gossip_processor.poll_ready(cx));

        let stream = Pin::new(&mut self.inbound.gossip);
        let maybe_gossip = ready!(stream.poll_next(cx));
        match maybe_gossip {
            Some(Ok(gossip)) => {
                self.gossip_processor
                    .start_processing_item(gossip)
                    .map_err(|e| {
                        debug!(self.logger, "failed to process gossip"; "error" => ?e);
                    })?;
                Ok(Continue).into()
            }
            None => {
                debug!(self.logger, "gossip subscription ended by the peer");
                Ok(Disconnect).into()
            }
            Some(Err(e)) => {
                debug!(
                    self.logger,
                    "gossip stream failure";
                    "error" => %e,
                );
                Err(()).into()
            }
        }
    }
}

impl Future for Client {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        use self::ProcessingOutcome::*;

        loop {
            let mut progress = Progress(None);

            progress.update(self.process_block_event(cx)?);
            progress.update(self.process_fragments(cx)?);
            progress.update(self.process_gossip(cx)?);

            // Block solicitations and chain pulls are special:
            // they are handled with client requests on the client side,
            // but on the server side, they are fed into the block event stream.
            progress.update(Pin::new(&mut self.block_solicitations).poll_next(cx).map(
                |maybe_item| match maybe_item {
                    Some(block_ids) => {
                        self.solicit_blocks(block_ids);
                        Continue
                    }
                    None => {
                        debug!(self.logger, "outbound block solicitation stream closed");
                        Disconnect
                    }
                },
            ));
            progress.update(
                Pin::new(&mut self.chain_pulls)
                    .poll_next(cx)
                    .map(|maybe_item| match maybe_item {
                        Some(req) => {
                            self.pull_headers(req);
                            Continue
                        }
                        None => {
                            debug!(self.logger, "outbound header pull stream closed");
                            Disconnect
                        }
                    }),
            );

            match progress {
                Progress(None) => return Poll::Pending,
                Progress(Some(Continue)) => continue,
                Progress(Some(Disconnect)) => {
                    info!(self.logger, "disconnecting client");
                    return ().into();
                }
            }
        }
    }
}
