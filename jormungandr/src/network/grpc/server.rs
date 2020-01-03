use super::super::{service::NodeService, Channels, GlobalStateR, ListenError};
use crate::settings::start::network::Listen;
use network_grpc::server::{self, TcpListen};

use futures::stream::FuturesUnordered;
use slog::Logger;
use tokio::net::TcpStream;
use tokio::prelude::*;

use std::net::SocketAddr;

type Server = server::Server<NodeService>;

pub fn run_listen_socket(
    listen: &Listen,
    state: GlobalStateR,
    channels: Channels,
) -> Result<impl Future<Item = (), Error = ()>, ListenError> {
    let sockaddr = listen.address();

    let logger = state.logger().new(o!("local_addr" => sockaddr.to_string()));
    info!(logger, "listening and accepting gRPC connections");

    match server::listen(&sockaddr) {
        Err(e) => Err(ListenError { cause: e, sockaddr }),
        Ok(listen) => {
            let capacity = state.config.max_connections;
            let node_server = NodeService::new(channels, state);
            let server = Server::new(node_server);

            let conn_mgr = Connections {
                listen,
                server,
                capacity,
                conn_set: FuturesUnordered::new(),
                logger: logger.clone(),
            };

            Ok(conn_mgr)
        }
    }
}

struct Connection {
    inner: server::Connection,
    logger: Logger,
}

impl Connection {
    fn serve(
        server: &mut Server,
        stream: TcpStream,
        peer_addr: SocketAddr,
        logger: &Logger,
    ) -> Self {
        // FIXME: obtain the peer address from the listener stream
        let logger = logger.new(o!("peer_addr" => peer_addr));
        info!(logger, "accepted connection");
        Connection {
            inner: server.serve(stream),
            logger,
        }
    }
}

impl Future for Connection {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        use network_grpc::server::Error;

        try_ready!(self.inner.poll().map_err(|e| match e {
            Error::Protocol(e) => {
                info!(
                    self.logger,
                    "incoming HTTP/2 connection error";
                    "reason" => %e,
                );
            }
            e => {
                warn!(
                    self.logger,
                    "incoming connection failed";
                    "error" => ?e,
                );
            }
        }));
        info!(self.logger, "connection closed");
        Ok(Async::Ready(()))
    }
}

struct Connections {
    listen: TcpListen,
    server: Server,
    capacity: usize,
    conn_set: FuturesUnordered<Connection>,
    logger: Logger,
}

impl Future for Connections {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        loop {
            if !self.conn_set.is_empty() {
                match self.conn_set.poll() {
                    Ok(Async::NotReady) => {
                        // Punt to self.listen.poll() below
                    }
                    Ok(Async::Ready(None)) => {}
                    Ok(Async::Ready(Some(()))) => debug!(
                        self.logger,
                        "a client peer connection has been closed";
                        "active_connections" => self.conn_set.len(),
                    ),
                    Err(()) => {}
                }
            }
            match self.listen.poll() {
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Ok(Async::Ready(Some((stream, peer_addr)))) => {
                    if self.conn_set.len() < self.capacity {
                        let conn =
                            Connection::serve(&mut self.server, stream, peer_addr, &self.logger);
                        self.conn_set.push(conn);
                    } else {
                        // The pool of managed connections is full.
                        // Reject this connection by dropping the stream,
                        // which is the only portable way to close the file
                        // descriptor.
                    }
                }
                Ok(Async::Ready(None)) => {
                    info!(self.logger, "listening socket has closed");
                    return Ok(Async::Ready(()));
                }
                Err(e) => {
                    error!(
                        self.logger,
                        "error while accepting connection";
                        "reason" => %e,
                    );
                    return Err(());
                }
            }
        }
    }
}
