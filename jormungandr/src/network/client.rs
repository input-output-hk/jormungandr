use super::{
    grpc,
    p2p::{
        comm::{ClientCommand, PeerComms, Subscription},
        topology,
    },
    subscription, Channels, ConnectionState,
};
use crate::{
    blockcfg::{Block, Header, HeaderHash},
    intercom::{self, BlockMsg, ClientMsg},
};
use futures::prelude::*;
use futures::sync::mpsc;
use network_core::{
    client::{self as core_client, block::BlockService, gossip::GossipService, p2p::P2pService},
    gossip::Node,
    subscription::BlockEvent,
};
use slog::Logger;

// Length of the channel buffering commands for client connections to the peers.
const COMMAND_BUFFER_LEN: usize = 32;

pub struct Client<S>
where
    S: BlockService,
{
    service: S,
    commands: mpsc::Receiver<ClientCommand>,
    channels: Channels,
    remote_node_id: topology::NodeId,
    block_events: S::BlockSubscription,
    block_solicitations: Subscription<Vec<HeaderHash>>,
    logger: Logger,
}

impl<S: BlockService> Client<S> {
    pub fn remote_node_id(&self) -> topology::NodeId {
        self.remote_node_id
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }
}

impl<S> Client<S>
where
    S: core_client::Client,
    S: P2pService<NodeId = topology::NodeId>,
    S: BlockService<Block = Block>,
    S: GossipService<Node = topology::Node>,
    S::UploadBlocksFuture: Send + 'static,
    S::GossipSubscription: Send + 'static,
{
    fn subscribe(
        service: S,
        state: ConnectionState,
        channels: Channels,
    ) -> impl Future<Item = (Self, PeerComms), Error = ()> {
        let (commands_tx, commands_rx) = mpsc::channel(COMMAND_BUFFER_LEN);
        let mut peer_comms = PeerComms::client(commands_tx);
        let err_logger = state.logger().clone();
        service
            .ready()
            .and_then(move |mut service| {
                let block_req =
                    service.block_subscription(peer_comms.subscribe_to_block_announcements());
                service
                    .ready()
                    .map(move |service| (service, peer_comms, block_req))
            })
            .and_then(move |(mut service, mut peer_comms, block_req)| {
                let gossip_req = service.gossip_subscription(peer_comms.subscribe_to_gossip());
                block_req
                    .join(gossip_req)
                    .map(move |(block_res, gossip_res)| {
                        (service, peer_comms, block_res, gossip_res)
                    })
            })
            .map_err(move |err| {
                warn!(err_logger, "subscription request failed: {:?}", err);
            })
            .and_then(
                move |(
                    service,
                    mut peer_comms,
                    (block_events, node_id),
                    (gossip_sub, node_id_1),
                )| {
                    if node_id != node_id_1 {
                        warn!(
                            state.logger(),
                            "peer subscription IDs do not match: {} != {}", node_id, node_id_1
                        );
                        return Err(());
                    }
                    let client_logger = state.logger().new(o!("node_id" => node_id.0.as_u128()));

                    // Spin off processing tasks for subscriptions that can be
                    // managed with just the global state.
                    subscription::process_gossip(gossip_sub, state.global, client_logger.clone());

                    // Plug the block solicitations to be handled
                    // via client requests.
                    let block_solicitations = peer_comms.subscribe_to_block_solicitations();

                    // Resolve with the client instance and communication handles.
                    let client = Client {
                        service,
                        commands: commands_rx,
                        channels,
                        remote_node_id: node_id,
                        block_events,
                        block_solicitations,
                        logger: client_logger,
                    };
                    Ok((client, peer_comms))
                },
            )
    }
}

impl<S> Client<S>
where
    S: BlockService<Block = Block>,
    S::PushHeadersFuture: Send + 'static,
    S::UploadBlocksFuture: Send + 'static,
{
    fn process_block_event(&mut self, event: BlockEvent<S::Block>) {
        match event {
            BlockEvent::Announce(header) => {
                self.channels
                    .block_box
                    .try_send(BlockMsg::AnnouncedBlock(header, self.remote_node_id))
                    .unwrap();
            }
            BlockEvent::Solicit(block_ids) => {
                let (reply_handle, stream) = intercom::stream_reply::<
                    Block,
                    network_core::error::Error,
                >(self.logger.clone());
                self.channels
                    .client_box
                    .send_to(ClientMsg::GetBlocks(block_ids, reply_handle));
                let node_id = self.remote_node_id;
                let done_logger = self.logger.clone();
                let err_logger = self.logger.clone();
                tokio::spawn(
                    self.service
                        .upload_blocks(stream)
                        .map(move |_| {
                            debug!(done_logger, "finished uploading blocks to {}", node_id);
                        })
                        .map_err(move |err| {
                            warn!(err_logger, "UploadBlocks request failed: {:?}", err);
                        }),
                );
            }
            BlockEvent::Missing { from, to } => {
                let (reply_handle, stream) = intercom::stream_reply::<
                    Header,
                    network_core::error::Error,
                >(self.logger.clone());
                self.channels.client_box.send_to(ClientMsg::GetHeadersRange(
                    from,
                    to,
                    reply_handle,
                ));
                let node_id = self.remote_node_id;
                let done_logger = self.logger.clone();
                let err_logger = self.logger.clone();
                tokio::spawn(
                    self.service
                        .push_headers(stream)
                        .map(move |_| {
                            debug!(done_logger, "finished pushing headers to {}", node_id);
                        })
                        .map_err(move |err| {
                            warn!(err_logger, "PushHeaders request failed: {:?}", err);
                        }),
                );
            }
        }
    }
}

impl<S> Client<S>
where
    S: BlockService<Block = Block>,
    S::PullHeadersFuture: Send + 'static,
    S::PullHeadersStream: Send + 'static,
{
    fn process_command(&mut self, command: ClientCommand) {
        match command {
            ClientCommand::PullHeaders { from, to } => {
                let mut block_box = self.channels.block_box.clone();
                let node_id = self.remote_node_id.clone();
                let err_logger = self.logger.clone();
                let and_then_logger = self.logger.clone();
                tokio::spawn(
                    self.service
                        .pull_headers(&from, &to)
                        .map_err(move |e| {
                            warn!(
                                err_logger,
                                "solicitation request PullHeaders failed: {:?}", e
                            );
                        })
                        .and_then(move |headers| {
                            // FIXME: batch headers and send them to the
                            // processing task in a new type of message
                            // specialized for restoration of the chain.
                            headers
                                .for_each(move |header| {
                                    block_box
                                        .try_send(BlockMsg::AnnouncedBlock(header, node_id))
                                        .unwrap();
                                    Ok(())
                                })
                                .map_err(move |e| {
                                    warn!(
                                        and_then_logger,
                                        "solicitation stream response to GetBlocks failed: {:?}", e
                                    );
                                })
                        }),
                );
            }
        }
    }
}

impl<S> Client<S>
where
    S: BlockService<Block = Block>,
    S::GetBlocksFuture: Send + 'static,
    S::GetBlocksStream: Send + 'static,
{
    fn solicit_blocks(&mut self, block_ids: &[HeaderHash]) {
        let mut block_box = self.channels.block_box.clone();
        let err_logger = self.logger.clone();
        let and_then_logger = self.logger.clone();
        tokio::spawn(
            self.service
                .get_blocks(block_ids)
                .map_err(move |e| {
                    warn!(err_logger, "solicitation request GetBlocks failed: {:?}", e);
                })
                .and_then(move |blocks| {
                    blocks
                        .for_each(move |block| {
                            block_box.try_send(BlockMsg::NetworkBlock(block)).unwrap();
                            Ok(())
                        })
                        .map_err(move |e| {
                            warn!(
                                and_then_logger,
                                "solicitation stream response to GetBlocks failed: {:?}", e
                            );
                        })
                }),
        );
    }
}

impl<S> Future for Client<S>
where
    S: BlockService<Block = Block>,
    S::GetBlocksFuture: Send + 'static,
    S::GetBlocksStream: Send + 'static,
    S::PullHeadersFuture: Send + 'static,
    S::PullHeadersStream: Send + 'static,
    S::PushHeadersFuture: Send + 'static,
    S::UploadBlocksFuture: Send + 'static,
{
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<(), ()> {
        loop {
            let mut streams_ready = false;
            let block_event_polled = self.block_events.poll().map_err(|e| {
                info!(self.logger, "block subscription stream failure: {:?}", e);
            })?;
            match block_event_polled {
                Async::NotReady => {}
                Async::Ready(None) => {
                    debug!(self.logger, "block subscription stream terminated");
                    return Ok(().into());
                }
                Async::Ready(Some(event)) => {
                    streams_ready = true;
                    self.process_block_event(event);
                }
            }
            match self.commands.poll().unwrap() {
                Async::NotReady => {}
                Async::Ready(None) => {
                    debug!(self.logger, "client command stream closed");
                    return Ok(().into());
                }
                Async::Ready(Some(command)) => {
                    streams_ready = true;
                    self.process_command(command);
                }
            }
            // Block solicitations are special: they are handled with
            // client requests on the client side, but on the server side,
            // they are fed into the block event stream.
            match self.block_solicitations.poll().unwrap() {
                Async::NotReady => {}
                Async::Ready(None) => {
                    debug!(self.logger, "outbound block solicitation stream closed");
                    return Ok(().into());
                }
                Async::Ready(Some(block_ids)) => {
                    streams_ready = true;
                    self.solicit_blocks(&block_ids);
                }
            }
            if !streams_ready {
                return Ok(Async::NotReady);
            }
        }
    }
}

pub fn connect(
    state: ConnectionState,
    channels: Channels,
) -> impl Future<Item = (Client<grpc::Connection>, PeerComms), Error = ()> {
    let addr = state.connection;
    let err_logger = state.logger().clone();
    grpc::connect(addr, Some(state.global.as_ref().node.id()))
        .map_err(move |err| {
            warn!(
                err_logger,
                "error connecting to peer at {}: {:?}", addr, err
            );
        })
        .and_then(move |conn| Client::subscribe(conn, state, channels))
        .map(move |(client, comms)| {
            debug!(client.logger(), "connected to peer");
            (client, comms)
        })
}
