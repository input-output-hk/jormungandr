use super::{ConnectionState, GlobalState, SubscriptionId};
use crate::blockcfg::cardano::{self, Cardano};
use crate::intercom::{ClientMsg, Error, Reply, StreamReply, TransactionMsg};
use ::protocol::{
    network_transport::LightWeightConnectionId, protocol, Connection, Inbound, Message,
};
use crate::settings::network::{self, Listen, Peer};

use futures::prelude::*;
use futures::{
    future,
    stream::Stream,
    sync::mpsc::{self, UnboundedSender},
};
use tokio::net::{TcpListener, TcpStream};

use std::net::SocketAddr;

/// Simple RAII for the reply information to NTT protocol commands
#[derive(Clone, Debug)]
pub struct ReplyHandle {
    // the identifier of the connection we are replying to
    identifier: LightWeightConnectionId,
    // the appropriate sink to send the messages to
    sink: UnboundedSender<protocol::Message>,
    closed: bool,
}

impl ReplyHandle {
    pub fn new(
        identifier: LightWeightConnectionId,
        sink: UnboundedSender<protocol::Message>,
    ) -> Self {
        ReplyHandle {
            identifier,
            sink,
            closed: false,
        }
    }

    fn send_message(&self, message: protocol::Message) {
        debug_assert!(!self.closed);
        self.sink.unbounded_send(message).unwrap();
    }

    fn send_close(&mut self) {
        debug_assert!(!self.closed);
        self.sink
            .unbounded_send(protocol::Message::CloseConnection(self.identifier))
            .unwrap();
        self.closed = true;
    }
}

impl Drop for ReplyHandle {
    fn drop(&mut self) {
        if !self.closed {
            warn!("protocol reply was not properly finalized");
            self.sink
                .unbounded_send(protocol::Message::CloseConnection(self.identifier))
                .unwrap_or_default();
        }
    }
}

impl Reply<Vec<cardano::Header>> for ReplyHandle {
    fn reply_ok(&mut self, item: Vec<cardano::Header>) {
        self.send_message(protocol::Message::BlockHeaders(
            self.identifier,
            protocol::Response::Ok(item.into()),
        ));
        self.send_close();
    }

    fn reply_error(&mut self, error: Error) {
        self.send_message(protocol::Message::BlockHeaders(
            self.identifier,
            protocol::Response::Err(error.to_string()),
        ));
        self.send_close();
    }
}

impl Reply<cardano::Header> for ReplyHandle {
    fn reply_ok(&mut self, item: cardano::Header) {
        self.send_message(protocol::Message::BlockHeaders(
            self.identifier,
            protocol::Response::Ok(protocol::BlockHeaders(vec![item])),
        ));
        self.send_close();
    }

    fn reply_error(&mut self, error: Error) {
        self.send_message(protocol::Message::BlockHeaders(
            self.identifier,
            protocol::Response::Err(error.to_string()),
        ));
        self.send_close();
    }
}

impl StreamReply<cardano::Block> for ReplyHandle {
    fn send(&mut self, blk: cardano::Block) {
        self.send_message(protocol::Message::Block(
            self.identifier,
            protocol::Response::Ok(blk),
        ));
    }

    fn send_error(&mut self, error: Error) {
        self.send_message(protocol::Message::Block(
            self.identifier,
            protocol::Response::Err(error.to_string()),
        ));
    }

    fn close(&mut self) {
        self.send_close()
    }
}

pub fn run_listen_socket(
    sockaddr: SocketAddr,
    listen: Listen,
    state: GlobalState<Cardano>,
) -> tokio::executor::Spawn {
    let state = ConnectionState::new_listen(&state, listen);

    info!(
        "start listening and accepting connection to {}",
        state.connection
    );
    let server = TcpListener::bind(&sockaddr)
        .unwrap() // TODO, handle on error to provide better error message
        .incoming()
        .map_err(move |err| {
            // error while receiving an incoming connection
            // here we might need to log the error and try
            // to listen again on the sockaddr
            error!(
                "Error while accepting connection from {:?}: {:?}",
                sockaddr, err
            )
        })
        .for_each(move |stream| {
            // received incoming connection
            info!(
                "{} connected to {}",
                stream.peer_addr().unwrap(),
                stream.local_addr().unwrap()
            );
            let state = state
                .clone()
                .connected(network::Connection::Tcp(stream.peer_addr().unwrap()));
            Connection::accept(stream)
                .map_err(move |err| {
                    error!("Rejecting NTT connection from {:?}: {:?}", sockaddr, err)
                })
                .and_then(move |connection| {
                    let state = state.clone();
                    tokio::spawn(run_connection(state, connection))
                })
        });
    tokio::spawn(server)
}

pub fn run_connect_socket(
    sockaddr: SocketAddr,
    peer: Peer,
    state: GlobalState<Cardano>,
) -> tokio::executor::Spawn {
    let state = ConnectionState::new_peer(&state, peer);

    info!("connecting to {}", state.connection);
    let server = TcpStream::connect(&sockaddr)
        .map_err(move |err| error!("Error while connecting to {:?}: {:?}", sockaddr, err))
        .and_then(move |stream| {
            let state = state
                .clone()
                .connected(network::Connection::Tcp(stream.local_addr().unwrap()));
            info!(
                "{} connected to {}",
                stream.local_addr().unwrap(),
                stream.peer_addr().unwrap()
            );
            Connection::connect(stream)
                .map_err(move |err| {
                    error!("Rejecting NTT connection from {:?}: {:?}", sockaddr, err)
                })
                .and_then(move |connection| {
                    let state = state.clone();
                    tokio::spawn(run_connection(state, connection))
                })
        });
    tokio::spawn(server)
}

fn run_connection<T>(
    state: ConnectionState<Cardano>,
    connection: Connection<T>,
) -> impl future::Future<Item = (), Error = ()>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite,
{
    let (sink, stream) = connection.split();

    let (sink_tx, sink_rx) = mpsc::unbounded();

    let stream = stream
        .for_each(move |inbound| {
            debug!("[{}] inbound: {:#?}", state.connection, inbound);
            match inbound {
                Inbound::NothingExciting => {}
                Inbound::Block(lwcid, _block) => {
                    info!("received block from {}{:?}", state.connection, lwcid);
                }
                Inbound::NewConnection(lwcid) => {
                    debug!("new light connection {:?}", lwcid);
                }
                Inbound::NewNode(lwcid, node_id) => {
                    sink_tx
                        .unbounded_send(Message::AckNodeId(lwcid, node_id))
                        .unwrap();
                }
                Inbound::Subscribe(lwcid, _keep_alive) => {
                    // add the subscription of this LWCID in the loop, the duplicates will be silently
                    // replaced for now. we might want to report the error to the client in future
                    //
                    state.subscriptions.write().unwrap().insert(
                        SubscriptionId(state.connection.clone(), lwcid),
                        sink_tx.clone(),
                    );
                }
                Inbound::GetBlockHeaders(lwcid, get_block_header) => {
                    let handler = Box::new(ReplyHandle::new(lwcid, sink_tx.clone()));
                    if let Some(to) = get_block_header.to {
                        state
                            .channels
                            .client_box
                            .send_to(ClientMsg::GetBlockHeaders(
                                get_block_header.from,
                                to,
                                handler,
                            ));
                    } else {
                        state
                            .channels
                            .client_box
                            .send_to(ClientMsg::GetBlockTip(handler));
                    }
                }
                Inbound::GetBlocks(lwcid, get_blocks) => {
                    let handler = Box::new(ReplyHandle::new(lwcid, sink_tx.clone()));
                    state.channels.client_box.send_to(ClientMsg::GetBlocks(
                        get_blocks.from,
                        get_blocks.to,
                        handler,
                    ));
                }
                Inbound::SendTransaction(_lwcid, tx) => state
                    .channels
                    .transaction_box
                    .send_to(TransactionMsg::SendTransaction(vec![tx])),
                inbound => {
                    error!("unrecognized message {:#?}", inbound);
                }
            }
            future::ok(())
        })
        .map_err(|err| error!("connection stream error {:#?}", err));

    let sink = sink
        .subscribe(false)
        .map_err(|err| error!("cannot subscribe {:#?}", err))
        .and_then(move |(_lwcid, sink)| {
            sink_rx
                .fold(sink, |sink, outbound| {
                    // debug!("[{}] outbound: {:?}", state.connection, outbound);
                    match outbound {
                        Message::AckNodeId(_lwcid, node_id) => future::Either::A(
                            sink.ack_node_id(node_id)
                                .map_err(|err| error!("err {:?}", err)),
                        ),
                        message => future::Either::B(
                            sink.send(message).map_err(|err| error!("err {:?}", err)),
                        ),
                    }
                })
                .map(|_| ())
        }); // .map_err(|err| { error!("failed to subscribe: {:#?}", err) });

    stream.select(sink).then(|_| {
        info!("closing connection");
        Ok(())
    })
}
