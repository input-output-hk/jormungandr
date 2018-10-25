//! all the network related actions and processes
//!
//! This module only provides and handle the different connections
//! and act as message passing between the other modules (blockchain,
//! transactions...);
//!

use std::net::{SocketAddr};

use tokio::net::{TcpListener};
use protocol::{Inbound, Message, Connection};
use futures::{future, stream, sync::mpsc, prelude::{*}};
use intercom::{ClientMsg, TransactionMsg, BlockMsg};

use utils::task::{TaskMessageBox};
use settings::network::{self, Peer, Listen};

type TODO = u32;

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

    let peer_iter = stream::iter_ok(config.peer_nodes).for_each(move |peer| {
        match peer.connection {
            network::Connection::Socket(sockaddr) => {
                run_listen_socket(sockaddr, peer, state.clone())
            },
            #[cfg(unix)]
            network::Connection::Unix(path) => unimplemented!()
        }
    });

    tokio::run(peer_iter);
}

fn run_listen_socket(sockaddr: SocketAddr, peer: Peer, state: State)
    -> tokio::executor::Spawn
{
    let server = TcpListener::bind(&sockaddr).unwrap().incoming()
        .map_err(move |err| {
            error!("Error while accepting connection from {:?}: {:?}", sockaddr, err)
        }).for_each(move |stream| {
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
                println!("inbound: {:?}", inbound);
            }
        }
        future::ok(())
    }).map_err(|err| {
        println!("connection stream error {:#?}", err)
    });

    let sink = sink_rx.fold(sink, |sink, outbound| {
        match outbound {
            Message::AckNodeId(_lwcid, node_id) => {
                future::Either::A(sink.ack_node_id(node_id)
                    .map_err(|err| println!("err {:?}", err)))
            },
            message => future::Either::B(sink.send(message)
                    .map_err(|err| println!("err {:?}", err)))
        }
    }).map(|_| ());

    stream.select(sink)
        .then(|_| { println!("closing connection"); Ok(()) })
}
