use super::super::{service::NodeService, Channels, GlobalStateR};
use crate::settings::start::network::Listen;
use network_grpc::server::{self, Server};

use futures::future::Either;
use tokio::prelude::*;

pub fn run_listen_socket(
    listen: Listen,
    state: GlobalStateR,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
    let sockaddr = listen.address();

    info!(
        state.logger(),
        "start listening and accepting gRPC connections on {}", sockaddr
    );

    match server::listen(&sockaddr) {
        Err(error) => {
            error!(
                state.logger(),
                "Error while listening on {}: {}", sockaddr, error
            );
            Either::A(future::err(()))
        }
        Ok(listener_stream) => {
            let fold_logger = state.logger().clone();
            let err_logger = state.logger().clone();
            let node_server = NodeService::new(channels, state);
            let server = Server::new(node_server);

            let future = listener_stream
                .map_err(move |err| {
                    // error while receiving an incoming connection
                    // here we might need to log the error and try
                    // to listen again on the sockaddr
                    error!(
                        err_logger,
                        "Error while accepting connection on {}: {:?}", sockaddr, err
                    );
                })
                .fold(server, move |mut server, stream| {
                    // received incoming connection
                    let conn_logger = match stream.peer_addr() {
                        Ok(addr) => fold_logger.new(o!("peer_addr" => addr)),
                        Err(e) => {
                            debug!(
                                fold_logger,
                                "connection rejected because peer address can't be obtained";
                                "reason" => %e);
                            return Ok(server)
                        },
                    };
                    info!(
                        conn_logger,
                        "incoming P2P connection on {}",
                        stream.local_addr().unwrap(),
                    );

                    let conn = server.serve(stream);
                    tokio::spawn(
                        conn.then(move |res| {
                            use network_grpc::server::Error;

                            match res {
                                Ok(()) => {
                                    info!(conn_logger, "incoming P2P connection closed");
                                }
                                Err(Error::Protocol(e)) => {
                                    info!(conn_logger, "incoming P2P HTTP/2 connection error"; "reason" => %e);
                                }
                                Err(e) => {
                                    warn!(conn_logger, "incoming P2P connection failed"; "error" => ?e);
                                }
                            }
                            Ok(())
                        })
                    );

                    Ok(server)
                })
                .map(|_| ());

            Either::B(future)
        }
    }
}
