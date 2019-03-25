use crate::blockcfg::BlockConfig;
use crate::network::{service::ConnectionServices, ConnectionState, GlobalState};
use crate::settings::start::network::Listen;

use network_grpc::server::{listen, Server};

use futures::future;
use futures::prelude::*;
use tokio::{executor::DefaultExecutor, sync::mpsc};

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
    listen_to: Listen,
    state: GlobalState<B>,
) -> impl Future<Item = (), Error = ()>
where
    B: BlockConfig + 'static,
{
    let (block_sender, block_receiver) = mpsc::unbounded_channel();
    let state = ConnectionState::new_listen(&state, &listen_to, block_sender);

    info!(
        "start listening and accepting gRPC connections on {}",
        sockaddr
    );

    match listen(&sockaddr) {
        Err(error) => {
            error!("Error while listening to {}", error ; sockaddr = sockaddr);
            unimplemented!()
        }
        Ok(listener_stream) => {
            let node_services = ConnectionServices::new(state);
            let server = Server::new(node_services, DefaultExecutor::current());

            listener_stream
                .map_err(move |err| {
                    // error while receiving an incoming connection
                    // here we might need to log the error and try
                    // to listen again on the sockaddr
                    error!(
                        "Error while accepting connection on {}: {:?}",
                        sockaddr, err
                    );
                })
                .fold(server, |mut server, stream| {
                    // received incoming connection
                    info!(
                        "{} connected to {}",
                        stream.peer_addr().unwrap(),
                        stream.local_addr().unwrap(),
                    );

                    let conn = server.serve(stream);

                    tokio::spawn(conn.map_err(|e| error!("server error: {:?}", e)));

                    future::ok(server)
                })
                .map(|_| ())
        }
    }
}
