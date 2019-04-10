use super::super::{service::NodeService, Channels, GlobalStateR};
use crate::settings::start::network::Listen;

use network_grpc::server::{self, Server};

use tokio::executor::DefaultExecutor;
use tokio::prelude::*;

pub fn run_listen_socket(
    listen: Listen,
    state: GlobalStateR,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
    let sockaddr = listen.address();

    info!(
        "start listening and accepting gRPC connections on {}",
        sockaddr
    );

    match server::listen(&sockaddr) {
        Err(error) => {
            error!("Error while listening to {}", error ; sockaddr = sockaddr);
            unimplemented!()
        }
        Ok(listener_stream) => {
            let node_server = NodeService::new(channels, state);
            let server = Server::new(node_server, DefaultExecutor::current());

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
