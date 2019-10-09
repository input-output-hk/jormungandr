use super::super::{service::NodeService, Channels, GlobalStateR, ListenError};
use crate::settings::start::network::Listen;
use network_grpc::server::{self, Server};

use tk_listen::ListenExt;
use tokio::prelude::*;

pub fn run_listen_socket(
    listen: &Listen,
    state: GlobalStateR,
    channels: Channels,
) -> Result<impl Future<Item = (), Error = ()>, ListenError> {
    let sockaddr = listen.address();

    info!(
        state.logger(),
        "start listening and accepting gRPC connections on {}", sockaddr
    );

    match server::listen(&sockaddr) {
        Err(e) => Err(ListenError { cause: e, sockaddr }),
        Ok(listener_stream) => {
            let max_connections = state.config.max_connections;
            let fold_logger = state.logger().clone();
            let err_logger = state.logger().clone();
            let node_server = NodeService::new(channels, state);
            let mut server = Server::new(node_server);

            let future = listener_stream
                .map_err(move |err| {
                    // Fatal error while receiving an incoming connection
                    error!(
                        err_logger,
                        "Error while accepting connection on {}: {:?}", sockaddr, err
                    );
                })
                .filter_map(move |stream| {
                    // received incoming connection
                    let conn_logger = match stream.peer_addr() {
                        Ok(addr) => fold_logger.new(o!("peer_addr" => addr)),
                        Err(e) => {
                            debug!(
                                fold_logger,
                                "connection rejected because peer address can't be obtained";
                                "reason" => %e);
                            return None;
                        }
                    };
                    info!(
                        conn_logger,
                        "incoming P2P connection on {}",
                        stream.local_addr().unwrap(),
                    );

                    let conn = server.serve(stream).then(move |res| {
                        use network_grpc::server::Error;

                        match res {
                            Ok(()) => {
                                info!(conn_logger, "incoming P2P connection closed");
                            }
                            Err(Error::Protocol(e)) => {
                                info!(
                                    conn_logger,
                                    "incoming P2P HTTP/2 connection error";
                                    "reason" => %e,
                                );
                            }
                            Err(e) => {
                                warn!(
                                    conn_logger,
                                    "incoming P2P connection failed";
                                    "error" => ?e,
                                );
                            }
                        }
                        Ok(())
                    });
                    Some(conn)
                })
                .listen(max_connections);

            Ok(future)
        }
    }
}
