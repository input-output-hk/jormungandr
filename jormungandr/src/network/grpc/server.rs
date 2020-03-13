use super::super::{service::NodeService, Channels, GlobalStateR, ListenError};
use crate::settings::start::network::Listen;
use chain_network::grpc::server::{self, TcpListen};

use futures::stream::FuturesUnordered;
use slog::Logger;
use tokio02::net::TcpStream;

use std::any::Any;
use std::net::SocketAddr;

type Server = server::Server<NodeService>;

pub async fn run_listen_socket(
    listen: &Listen,
    state: GlobalStateR,
    channels: Channels,
) -> Result<(), ListenError> {
    let sockaddr = listen.address();

    let logger = state.logger().new(o!("local_addr" => sockaddr.to_string()));
    info!(logger, "listening and accepting gRPC connections");

    match server::listen(&sockaddr) {
        Err(e) => Err(ListenError { cause: e, sockaddr }),
        Ok(listen) => {
            let capacity = state.config.max_connections;
            let node_server = NodeService::new(channels, state);
            let server = Server::new(node_server);
            let panic_logger = logger.clone();

            let thread_pool = tokio_threadpool::Builder::new()
                .name_prefix("server")
                .panic_handler(move |err| handle_task_panic(&err, &panic_logger))
                .build();

            let conn_mgr = Connections {
                listen,
                server,
                capacity,
                conn_set: FuturesUnordered::new(),
                thread_pool: Some(thread_pool),
                logger: logger.clone(),
            };

            Ok(conn_mgr.and_then(|shutdown| shutdown))
        }
    }
}

fn handle_task_panic(err: &(dyn Any + Send), logger: &Logger) {
    if let Some(msg) = err.downcast_ref::<String>() {
        crit!(logger, "server task panicked: {}", msg);
    } else {
        crit!(logger, "server task panicked");
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

        match self.inner.poll() {
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Ok(Async::Ready(())) => {
                info!(self.logger, "connection closed");
            }
            Err(Error::Protocol(e)) => {
                info!(
                    self.logger,
                    "incoming HTTP/2 connection error";
                    "reason" => %e,
                );
            }
            Err(e) => {
                warn!(
                    self.logger,
                    "incoming connection failed";
                    "error" => ?e,
                );
            }
        }

        Ok(Async::Ready(()))
    }
}

type ConnHandle = tokio_threadpool::SpawnHandle<(), ()>;

struct Connections {
    listen: TcpListen,
    server: Server,
    capacity: usize,
    conn_set: FuturesUnordered<ConnHandle>,
    thread_pool: Option<ThreadPool>,
    logger: Logger,
}

impl Future for Connections {
    type Item = Shutdown;
    type Error = ();

    fn poll(&mut self) -> Poll<Shutdown, ()> {
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
                        let thread_pool = self
                            .thread_pool
                            .as_ref()
                            .expect("server polled after shutdown");
                        let handle = thread_pool.spawn_handle(conn);
                        self.conn_set.push(handle);
                    } else {
                        // The pool of managed connections is full.
                        // Reject this connection by dropping the stream,
                        // which is the only portable way to close the file
                        // descriptor.
                    }
                }
                Ok(Async::Ready(None)) => {
                    // FIXME: this is never returned by the current
                    // implementation in network-grpc, so this code
                    // is a placeholder, to be reused for graceful
                    // service shutdown on a different pollable condition.
                    info!(self.logger, "listening socket has closed");
                    let thread_pool = self
                        .thread_pool
                        .take()
                        .expect("server polled after shutdown");
                    return Ok(Async::Ready(thread_pool.shutdown()));
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
