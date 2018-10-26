//! all the network related actions and processes
//!
//! This module only provides and handle the different connections
//! and act as message passing between the other modules (blockchain,
//! transactions...);
//!

use std::net::{SocketAddr};

use tokio::net::{TcpListener, TcpStream};
use protocol::{Inbound, Message, Connection};
use futures::{future, stream::{self, Stream}, sync::mpsc, prelude::{*}};
use intercom::{ClientMsg, TransactionMsg, BlockMsg};

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
pub struct State {
    pub config:   network::Configuration,
    pub channels: Channels,
}

pub fn run( config: network::Configuration
          , channels: Channels
          )
{
    let state = State {
        config:   config.clone(),
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

fn run_listen_socket(sockaddr: SocketAddr, listen: Listen, state: State)
    -> tokio::executor::Spawn
{
    info!("start listening and accepting connection to {}", listen.connection);
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
            let state = state.clone();
            Connection::accept(stream)
                .map_err(move |err| error!("Rejecting NTT connection from {:?}: {:?}", sockaddr, err))
                .and_then(move |connection| {
                    let state = state.clone();
                    tokio::spawn(run_connection(state, connection))
                })
        });
    tokio::spawn(server)
}

fn run_connect_socket(sockaddr: SocketAddr, peer: Peer, state: State)
    -> tokio::executor::Spawn
{
    info!("connecting to {}", peer.connection);
    let server = TcpStream::connect(&sockaddr)
        .map_err(move |err| {
            error!("Error while connecting to {:?}: {:?}", sockaddr, err)
        }).and_then(move |stream| {
            let state = state.clone();
            info!("{} connected to {}", stream.local_addr().unwrap(), stream.peer_addr().unwrap());
            Connection::accept(stream)
                .map_err(move |err| error!("Rejecting NTT connection from {:?}: {:?}", sockaddr, err))
                .and_then(move |connection| {
                    let state = state.clone();
                    tokio::spawn(run_connection(state, connection))
                })
        });
    tokio::spawn(server)
}


fn run_connection<T>(state: State, connection: Connection<T>)
    -> impl future::Future<Item = (), Error = ()>
  where T: tokio::io::AsyncRead + tokio::io::AsyncWrite
{
    let (sink, stream) = connection.split();

    let (sink_tx, sink_rx) = mpsc::unbounded();

    let stream = stream.for_each(move |inbound| {
        match inbound {
            Inbound::NewNode(lwcid, node_id) => {
                sink_tx.unbounded_send(Message::AckNodeId(lwcid, node_id)).unwrap();
            },
            inbound => {
                debug!("inbound: {:?}", inbound);
            }
        }
        future::ok(())
    }).map_err(|err| {
        error!("connection stream error {:#?}", err)
    });

    let sink = sink_rx.fold(sink, |sink, outbound| {
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
