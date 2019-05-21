use super::{
    grpc,
    p2p::{
        comm::{PeerComms, Subscription},
        topology,
    },
    subscription, Channels, ConnectionState, GlobalStateR,
};
use crate::{
    blockcfg::{Block, HeaderHash},
    intercom::{self, BlockMsg, ClientMsg},
};

use network_core::{
    client::{block::BlockService, gossip::GossipService, P2pService},
    subscription::BlockEvent,
};

use futures::prelude::*;

pub struct Client<S>
where
    S: BlockService,
{
    service: S,
    channels: Channels,
    remote_node_id: topology::NodeId,
    block_events: S::BlockSubscription,
    block_solicitations: Subscription<Vec<HeaderHash>>,
}

impl<S> Client<S>
where
    S: BlockService,
{
    pub fn remote_node_id(&self) -> topology::NodeId {
        self.remote_node_id
    }
}

impl<S> Client<S>
where
    S: P2pService<NodeId = topology::NodeId>,
    S: BlockService<Block = Block>,
    S: GossipService<Node = topology::Node>,
    S::UploadBlocksFuture: Send + 'static,
    S::GossipSubscription: Send + 'static,
{
    fn subscribe(
        mut service: S,
        global_state: GlobalStateR,
        channels: Channels,
    ) -> impl Future<Item = (Self, PeerComms), Error = ()> {
        let mut peer_comms = PeerComms::new();
        let block_req = service.block_subscription(peer_comms.subscribe_to_block_announcements());
        let gossip_req = service.gossip_subscription(peer_comms.subscribe_to_gossip());
        block_req
            .join(gossip_req)
            .map_err(move |err| {
                warn!("subscription request failed: {:?}", err);
            })
            .and_then(move |((block_events, node_id), (gossip_sub, node_id_1))| {
                if node_id != node_id_1 {
                    warn!(
                        "peer subscription IDs do not match: {} != {}",
                        node_id, node_id_1
                    );
                    return Err(());
                }

                // Spin off processing tasks for subscriptions that can be
                // managed with just the global state.
                subscription::process_gossip(gossip_sub, global_state);

                // Plug the block solicitations to be handled
                // via client requests.
                let block_solicitations = peer_comms.subscribe_to_block_solicitations();

                // Resolve with the client instance and communication handles.
                let client = Client {
                    service,
                    channels,
                    remote_node_id: node_id,
                    block_events,
                    block_solicitations,
                };
                Ok((client, peer_comms))
            })
    }
}

impl<S> Client<S>
where
    S: BlockService<Block = Block>,
    S::UploadBlocksFuture: Send + 'static,
{
    fn process_block_event(&mut self, event: BlockEvent<S::Block>) {
        match event {
            BlockEvent::Announce(header) => {
                self.channels
                    .block_box
                    .send(BlockMsg::AnnouncedBlock(header, self.remote_node_id));
            }
            BlockEvent::Solicit(block_ids) => {
                let (reply_handle, stream) =
                    intercom::stream_reply::<Block, network_core::error::Error>();
                self.channels
                    .client_box
                    .send_to(ClientMsg::GetBlocks(block_ids, reply_handle));
                let node_id = self.remote_node_id;
                tokio::spawn(
                    self.service
                        .upload_blocks(stream)
                        .map(move |_res| {
                            debug!("finished uploading blocks to {}", node_id);
                        })
                        .map_err(|err| {
                            warn!("UploadBlocks request failed: {:?}", err);
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
        tokio::spawn(
            self.service
                .get_blocks(block_ids)
                .map_err(|e| {
                    warn!("solicitation request GetBlocks failed: {:?}", e);
                })
                .and_then(|blocks| {
                    blocks
                        .for_each(move |block| {
                            block_box.send(BlockMsg::NetworkBlock(block));
                            Ok(())
                        })
                        .map_err(|e| {
                            warn!("solicitation stream response to GetBlocks failed: {:?}", e);
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
    S::UploadBlocksFuture: Send + 'static,
{
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<(), ()> {
        loop {
            let mut streams_ready = false;
            let block_event_polled = self.block_events.poll().map_err(|e| {
                info!("block subscription stream failure: {:?}", e);
            })?;
            match block_event_polled {
                Async::NotReady => {}
                Async::Ready(None) => {
                    debug!("block subscription stream terminated");
                    return Ok(().into());
                }
                Async::Ready(Some(event)) => {
                    streams_ready = true;
                    self.process_block_event(event);
                }
            }
            let block_solicitation_polled = self.block_solicitations.poll().unwrap();
            match block_solicitation_polled {
                Async::NotReady => {}
                Async::Ready(None) => {
                    debug!("outbound block solicitation stream closed");
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
    grpc::connect(&state)
        .map_err(move |err| {
            warn!("error connecting to peer at {}: {:?}", addr, err);
        })
        .and_then(move |conn| Client::subscribe(conn, state.global, channels))
        .map(move |(client, comms)| {
            debug!("connected to peer {} at {}", client.remote_node_id(), addr);
            (client, comms)
        })
}
