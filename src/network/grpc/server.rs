use super::super::{service::NodeServer, Channels, ConnectionState};

use network_grpc::server::{listen, Server};

use tokio::executor::DefaultExecutor;
use tokio::prelude::*;

use std::net::SocketAddr;

pub fn run_listen_socket(
    sockaddr: SocketAddr,
    state: ConnectionState,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
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
            let node_server = NodeServer::new(state, channels);
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
