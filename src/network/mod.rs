//! all the network related actions and processes
//!
//! This module only provides and handle the different connections
//! and act as message passing between the other modules (blockchain,
//! transactions...);
//!

use std::{net::{SocketAddr}, sync::{Arc}, time::{Duration}};

use tokio::net::{TcpListener, TcpStream};
use protocol::{Inbound, Message, Connection};
use futures::{future, stream::{self, Stream}, sync::mpsc, prelude::{*}};
use intercom::{ClientMsg, TransactionMsg, BlockMsg, NetworkHandler};

use utils::task::{TaskMessageBox};
use settings::network::{self, Peer, Listen};

/// all the different channels the network may need to talk to
#[derive(Clone)]
pub struct Channels {
    pub client_box:      TaskMessageBox<ClientMsg>,
    pub transaction_box: TaskMessageBox<TransactionMsg>,
    pub block_box:       TaskMessageBox<BlockMsg>,
}

#[derive(Clone)]
pub struct GlobalState {
    pub config:   Arc<network::Configuration>,
    pub channels: Channels,
}

#[derive(Clone)]
pub struct ConnectionState {
    /// The global network configuration
    pub global_network_configuration: Arc<network::Configuration>,

    /// the channels the connection will need to have to
    /// send messages too
    pub channels: Channels,

    /// the timeout to wait for unbefore the connection replies
    pub timeout: Duration,

    /// the local (to the task) connection details
    pub connection: network::Connection,

    pub connected: Option<network::Connection>,
}
impl ConnectionState {
    fn new_listen(global: &GlobalState, listen: Listen) -> Self {
        ConnectionState {
            global_network_configuration: global.config.clone(),
            channels: global.channels.clone(),
            timeout: listen.timeout,
            connection: listen.connection,
            connected: None,
        }
    }
    fn new_peer(global: &GlobalState, peer: Peer) -> Self {
        ConnectionState {
            global_network_configuration: global.config.clone(),
            channels: global.channels.clone(),
            timeout: peer.timeout,
            connection: peer.connection,
            connected: None,
        }
    }
    fn connected(mut self, connection: network::Connection) -> Self {
        self.connected = Some(connection);
        self
    }
}

pub fn run( config: network::Configuration
          , channels: Channels
          )
{
    let arc_config = Arc::new(config.clone());
    let state = GlobalState {
        config:   arc_config,
        channels: channels,
    };

    let state_listener = state.clone();
    // open the port for listenting/accepting other peers to connect too
    let listener = stream::iter_ok(config.listen_to).for_each(move |listen| {
        match listen.connection {
            network::Connection::Socket(sockaddr) => {
                run_listen_socket(sockaddr, listen, state_listener.clone())
            },
            #[cfg(unix)]
            network::Connection::Unix(path) => unimplemented!()
        }
    });

    let state_connection = state.clone();
    let connections = stream::iter_ok(config.peer_nodes).for_each(move |peer| {
        match peer.connection {
            network::Connection::Socket(sockaddr) => {
                run_connect_socket(sockaddr, peer, state_connection.clone())
            },
            #[cfg(unix)]
            network::Connection::Unix(path) => unimplemented!()
        }
    });

    tokio::run(connections.join(listener).map(|_| ()));
}

fn run_listen_socket(sockaddr: SocketAddr, listen: Listen, state: GlobalState)
    -> tokio::executor::Spawn
{
    let state = ConnectionState::new_listen(&state, listen);

    info!("start listening and accepting connection to {}", state.connection);
    let server = TcpListener::bind(&sockaddr)
        .unwrap() // TODO, handle on error to provide better error message
        .incoming()
        .map_err(move |err| {
            // error while receiving an incoming connection
            // here we might need to log the error and try
            // to listen again on the sockaddr
            error!("Error while accepting connection from {:?}: {:?}", sockaddr, err)
        }).for_each(move |stream| {
            // received incoming connection
            info!("{} connected to {}", stream.peer_addr().unwrap(), stream.local_addr().unwrap());
            let state = state.clone().connected(network::Connection::Socket(stream.peer_addr().unwrap()));
            Connection::accept(stream)
                .map_err(move |err| error!("Rejecting NTT connection from {:?}: {:?}", sockaddr, err))
                .and_then(move |connection| {
                    let state = state.clone();
                    tokio::spawn(run_connection(state, connection))
                })
        });
    tokio::spawn(server)
}

fn run_connect_socket(sockaddr: SocketAddr, peer: Peer, state: GlobalState)
    -> tokio::executor::Spawn
{
    let state = ConnectionState::new_peer(&state, peer);

    info!("connecting to {}", state.connection);
    let server = TcpStream::connect(&sockaddr)
        .map_err(move |err| {
            error!("Error while connecting to {:?}: {:?}", sockaddr, err)
        }).and_then(move |stream| {
            let state = state.clone().connected(network::Connection::Socket(stream.local_addr().unwrap()));
            info!("{} connected to {}", stream.local_addr().unwrap(), stream.peer_addr().unwrap());
            Connection::connect(stream)
                .map_err(move |err| error!("Rejecting NTT connection from {:?}: {:?}", sockaddr, err))
                .and_then(move |connection| {
                    let state = state.clone();
                    tokio::spawn(run_connection(state, connection))
                })
        });
    tokio::spawn(server)
}


fn run_connection<T>(state: ConnectionState, connection: Connection<T>)
    -> impl future::Future<Item = (), Error = ()>
  where T: tokio::io::AsyncRead + tokio::io::AsyncWrite
{
    let (sink, stream) = connection.split();

    let (sink_tx, sink_rx) = mpsc::unbounded();

    let stream = stream.for_each(move |inbound| {
        debug!("[{}] inbound: {:?}", state.connection, inbound);
        match inbound {
            Inbound::NewNode(lwcid, node_id) => {
                sink_tx.unbounded_send(Message::AckNodeId(lwcid, node_id)).unwrap();
            },
            Inbound::GetBlockHeaders(lwcid, get_block_header) => {
                let handler = NetworkHandler {
                    identifier: lwcid,
                    sink: sink_tx.clone(),
                    marker: std::marker::PhantomData,
                };
                if let Some(to) = get_block_header.to {
                    state.channels.client_box.send_to(
                        ClientMsg::GetBlockHeaders(get_block_header.from, to, handler)
                    );
                } else {
                    state.channels.client_box.send_to(
                        ClientMsg::GetBlockTip(handler)
                    );
                }
            }
            Inbound::GetBlocks(lwcid, get_blocks) => {
                state.channels.client_box.send_to(
                    ClientMsg::GetBlocks(
                        get_blocks.from,
                        get_blocks.to,
                        NetworkHandler {
                            identifier: lwcid,
                            sink: sink_tx.clone(),
                            marker: std::marker::PhantomData,
                        }
                    )
                );
            }
            inbound => {
            }
        }
        future::ok(())
    }).map_err(|err| {
        error!("connection stream error {:#?}", err)
    });

    let sink = sink_rx.fold(sink, |sink, outbound| {
        // debug!("[{}] outbound: {:?}", state.connection, outbound);
        match outbound {
            Message::AckNodeId(_lwcid, node_id) => {
                future::Either::A(sink.ack_node_id(node_id)
                    .map_err(|err| error!("err {:?}", err)))
            },
            message => future::Either::B(sink.send(message)
                    .map_err(|err| error!("err {:?}", err)))
        }
    }).map(|_| ());

    stream.select(sink)
        .then(|_| { info!("closing connection"); Ok(()) })
}
