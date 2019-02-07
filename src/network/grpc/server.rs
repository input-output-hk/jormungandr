use crate::blockcfg::{BlockConfig, Deserialize};
use crate::intercom::{self, ClientMsg};
use crate::network::{service::ConnectionServices, ConnectionState, GlobalState};
use crate::settings::network::Listen;

use chain_core::property;
use network_grpc::server::{listen, Server};

use futures::prelude::*;
use futures::{
    future::{self, FutureResult},
    sync::{mpsc, oneshot},
};
use tokio::{executor::DefaultExecutor, net::TcpListener};

use std::net::SocketAddr;

struct GrpcServer<B: BlockConfig> {
    state: ConnectionState<B>,
}

impl<B: BlockConfig> Clone for GrpcServer<B> {
    fn clone(&self) -> Self {
        GrpcServer {
            state: self.state.clone(),
        }
    }
}

pub fn run_listen_socket<B>(
    sockaddr: SocketAddr,
    listen: Listen,
    state: GlobalState<B>,
) -> tokio::executor::Spawn
where
    B: 'static + BlockConfig,
    <B as BlockConfig>::Block: Send,
    <B as BlockConfig>::BlockHash: Send,
    <B as BlockConfig>::Transaction: Send,
    <B as BlockConfig>::TransactionId: Send,
{
    let state = ConnectionState::new_listen(&state, listen);

    info!(
        "start listening and accepting gRPC connections on {}",
        sockaddr
    );

    let node_services = ConnectionServices::new(&state);
    let server = Server::new(node_services, DefaultExecutor::current());

    let listener = listen(&sockaddr)
        .unwrap() // TODO, handle on error to provide better error message
        .map_err(move |err| {
            // error while receiving an incoming connection
            // here we might need to log the error and try
            // to listen again on the sockaddr
            error!(
                "Error while accepting connection on {}: {:?}",
                sockaddr, err
            );
        })
        .and_then(|stream| {
            // received incoming connection
            info!(
                "{} connected to {}",
                stream.peer_addr().unwrap(),
                stream.local_addr().unwrap(),
            );

            let serve = server.serve(stream);

            tokio::spawn(serve.map_err(|e| error!("server error: {:?}", e)));
        });

    tokio::spawn(listener)
}
