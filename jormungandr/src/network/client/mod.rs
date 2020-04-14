mod connect;

use super::{
    buffer_sizes,
    grpc::{
        self,
        client::{BlockSubscription, FragmentSubscription, GossipSubscription},
    },
    p2p::{
        comm::{OutboundSubscription, PeerComms},
        Address,
    },
    subscription::GossipProcessor,
    Channels, GlobalStateR,
};
use crate::{
    blockcfg::{Block, Fragment, Header, HeaderHash},
    intercom::{self, BlockMsg, ClientMsg},
};
use chain_network::data::block::{BlockEvent, BlockIds, ChainPullRequest};
use chain_network::error as net_error;

use futures03::prelude::*;
use futures03::ready;
use slog::Logger;

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
    gossip_processor: GossipProcessor,
    channels: Channels,
    incoming_block_announcement: Option<Header>,
    incoming_solicitation: Option<ClientMsg>,
    incoming_fragment: Option<Fragment>,
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

        let gossip_processor = GossipProcessor::new(
            remote_node_id,
            global_state.clone(),
            logger.new(o!("stream" => "gossip", "direction" => "in")),
        );

        Client {
            service: inner,
            logger,
            global_state,
            inbound,
            block_solicitations: comms.subscribe_to_block_solicitations(),
            chain_pulls: comms.subscribe_to_chain_pulls(),
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
        let block_box = &mut self.channels.block_box;
        ready!(block_box.poll_ready(cx));
        if let Some(header) = self.incoming_block_announcement.take() {
            block_box
                .start_send(BlockMsg::AnnouncedBlock(header))
                .map_err(|e| {
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
            block_box.poll_flush(cx).map_err(|e| {
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
        let client_box = &mut self.channels.client_box;
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

        let maybe_event = ready!(self.inbound.block_events.poll_next(cx)).map_err(|e| {
            debug!(
                self.logger,
                "block subscription stream failure";
                "error" => ?e,
            );
        })?;
        let event = match maybe_event {
            Some(event) => event,
            None => {
                debug!(self.logger, "block event subscription ended by the peer");
                return Ok(Disconnect).into();
            }
        };
        match event {
            BlockEvent::Announce(header) => {
                debug_assert!(self.incoming_block_announcement.is_none());
                self.incoming_block_announcement = Some(header);
            }
            BlockEvent::Solicit(block_ids) => {
                self.upload_blocks(block_ids);
            }
            BlockEvent::Missing(req) => {
                self.push_missing_headers(req);
            }
        }
        Ok(Continue).into()
    }

    fn upload_blocks(&mut self, block_ids: Vec<HeaderHash>) {
        debug!(self.logger, "peer requests {} blocks", block_ids.len());
        let (reply_handle, stream) = intercom::stream_reply::<_, net_error::Error>(
            buffer_sizes::outbound::BLOCKS,
            self.logger.new(o!("solicitation" => "UploadBlocks")),
        );
        debug_assert!(self.incoming_solicitation.is_none());
        self.incoming_solicitation = Some(ClientMsg::GetBlocks(block_ids, reply_handle));
        let done_logger = self.logger.clone();
        let err_logger = self.logger.clone();
        self.global_state.spawn(
            self.service
                .upload_blocks(stream)
                .map(move |_| {
                    debug!(done_logger, "finished uploading blocks");
                })
                .map_err(move |e| {
                    info!(
                        err_logger,
                        "UploadBlocks request failed";
                        "error" => ?e,
                    );
                }),
        );
    }

    fn push_missing_headers(&mut self, req: ChainPullRequest) {
        debug!(
            self.logger,
            "peer requests missing part of the chain";
            "checkpoints" => ?req.from,
            "to" => ?req.to,
        );
        let (reply_handle, stream) = intercom::stream_reply::<_, net_error::Error>(
            buffer_sizes::outbound::HEADERS,
            self.logger.new(o!("solicitation" => "PushHeaders")),
        );
        debug_assert!(self.incoming_solicitation.is_none());
        self.incoming_solicitation =
            Some(ClientMsg::GetHeadersRange(req.from, req.to, reply_handle));
        let done_logger = self.logger.clone();
        let err_logger = self.logger.clone();
        self.global_state.spawn(
            self.service
                .push_headers(stream)
                .map(move |_| {
                    debug!(done_logger, "finished pushing headers");
                })
                .map_err(move |e| {
                    info!(
                        err_logger,
                        "PushHeaders request failed";
                        "error" => ?e,
                    );
                }),
        );
    }

    fn pull_headers(&mut self, req: ChainPullRequest) {
        let block_box = self.block_sink.message_box();
        let logger = self.logger.new(o!("request" => "PullHeaders"));
        let req_err_logger = logger.clone();
        let res_logger = logger.clone();
        let (handle, sink) = intercom::stream_request::<Header, (), net_error::Error>(
            buffer_sizes::inbound::HEADERS,
            logger.clone(),
        );
        // TODO: make sure that back pressure on the number of requests
        // in flight, imposed through self.service.poll_ready(),
        // prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state.spawn(
            block_box
                .send(BlockMsg::ChainHeaders(handle))
                .map_err(move |e| {
                    error!(
                        logger,
                        "failed to enqueue request for processing";
                        "reason" => %e,
                    );
                })
                .map(|_mbox| ()),
        );
        self.global_state.spawn(
            self.service
                .pull_headers(&req.from, &req.to)
                .map_err(move |e| {
                    info!(
                        req_err_logger,
                        "request failed";
                        "reason" => %e,
                    );
                })
                .and_then(move |stream| {
                    sink.send_all(stream)
                        .map_err(move |e| {
                            info!(
                                res_logger,
                                "response stream failed";
                                "reason" => %e,
                            );
                        })
                        .map(|_| ())
                }),
        );
    }

    fn solicit_blocks(&mut self, block_ids: &[HeaderHash]) {
        let block_box = self.block_sink.message_box();
        let logger = self.logger.new(o!("request" => "GetBlocks"));
        let req_err_logger = logger.clone();
        let res_logger = logger.clone();
        let (handle, sink) = intercom::stream_request::<Block, (), net_error::Error>(
            buffer_sizes::inbound::BLOCKS,
            logger.clone(),
        );
        // TODO: make sure that back pressure on the number of requests
        // in flight, imposed through self.service.poll_ready(),
        // prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state.spawn(
            block_box
                .send(BlockMsg::NetworkBlocks(handle))
                .map_err(move |e| {
                    error!(
                        logger,
                        "failed to enqueue request for processing";
                        "reason" => %e,
                    );
                })
                .map(|_mbox| ()),
        );
        self.global_state.spawn(
            self.service
                .get_blocks(block_ids)
                .map_err(move |e| {
                    info!(
                        req_err_logger,
                        "request failed";
                        "reason" => %e,
                    );
                })
                .and_then(move |stream| {
                    sink.send_all(stream)
                        .map_err(move |e| {
                            info!(
                                res_logger,
                                "response stream failed";
                                "reason" => %e,
                            );
                        })
                        .map(|_| ())
                }),
        );
    }

    fn process_fragments(&mut self, cx: &mut Context<'_>) -> Poll<Result<ProcessingOutcome, ()>> {
        use self::ProcessingOutcome::*;

        // Drive sending of a message to fragment task to completion
        // before polling more events from the fragment subscription
        // stream.
        let fragment_box = &mut self.channels.fragment_box;
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
            fragment_box.poll_flush(cx).map_err(|e| {
                error!(
                    self.logger,
                    "processing of incoming fragments failed";
                    "reason" => %e,
                );
            })?;
        }

        let maybe_fragment = ready!(self.inbound.fragments.poll_next(cx)).map_err(|e| {
            debug!(
                self.logger,
                "fragment stream failure";
                "error" => %e,
            );
        })?;
        match maybe_fragment {
            Some(fragment) => {
                debug_assert!(self.incoming_fragment.is_none());
                self.incoming_fragment = Some(fragment);
                Ok(Continue).into()
            }
            None => {
                debug!(self.logger, "fragment subscription ended by the peer");
                Ok(Disconnect).into()
            }
        }
    }

    fn process_gossip(&mut self, cx: &mut Context<'_>) -> Poll<Result<ProcessingOutcome, ()>> {
        use self::ProcessingOutcome::*;

        let maybe_gossip = ready!(self.inbound.gossip.poll_next(cx)).map_err(|e| {
            debug!(
                self.logger,
                "gossip stream failure";
                "error" => %e,
            );
        })?;
        match maybe_gossip {
            Some(gossip) => {
                self.gossip_processor.process_item(gossip);
                Ok(Continue).into()
            }
            None => {
                debug!(self.logger, "gossip subscription ended by the peer");
                Ok(Disconnect).into()
            }
        }
    }
}

impl Future for Client {
    type Output = ();

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        use self::ProcessingOutcome::*;

        loop {
            let mut progress = Progress(None);

            progress.update(self.process_block_event(cx)?);
            progress.update(self.process_fragments(cx)?);
            progress.update(self.process_gossip(cx)?);

            // Block solicitations and chain pulls are special:
            // they are handled with client requests on the client side,
            // but on the server side, they are fed into the block event stream.
            progress.update(self.block_solicitations.poll_next(cx).map(
                |maybe_item| match maybe_item {
                    Some(block_ids) => {
                        self.solicit_blocks(&block_ids);
                        Continue
                    }
                    None => {
                        debug!(self.logger, "outbound block solicitation stream closed");
                        Disconnect
                    }
                },
            ));
            progress.update(
                self.chain_pulls
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
                    return Ok(()).into();
                }
            }
        }
    }
}
