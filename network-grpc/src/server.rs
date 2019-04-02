use crate::{gen::node::server as gen_server, service::NodeService};

use network_core::server::{
    block::BlockService, content::ContentService, gossip::GossipService, Node,
};

use futures::future::Executor;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

use std::{error, fmt, net::SocketAddr};

#[cfg(unix)]
use std::path::Path;

/// The gRPC server for the blockchain node.
///
/// This type encapsulates the gRPC protocol server providing the
/// Node service. The application instantiates a `Server` wrapping a
/// blockchain service implementation satisfying the abstract network
/// service trait `Node`.
pub struct Server<T, E>
where
    T: Node + Clone,
    <T::BlockService as BlockService>::Header: Send + 'static,
    <T::ContentService as ContentService>::Message: Send + 'static,
    <T::GossipService as GossipService>::Node: Send + 'static,
    <T::GossipService as GossipService>::NodeId: Send + 'static,
{
    h2: tower_h2::Server<
        gen_server::NodeServer<NodeService<T>>,
        E,
        gen_server::node::ResponseBody<NodeService<T>>,
    >,
}

/// Connection of a client peer to the gRPC server.
pub struct Connection<S, T, E>
where
    S: AsyncRead + AsyncWrite,
    T: Node + Clone,
    <T::BlockService as BlockService>::Header: Send + 'static,
    <T::ContentService as ContentService>::Message: Send + 'static,
    <T::GossipService as GossipService>::Node: Send + 'static,
    <T::GossipService as GossipService>::NodeId: Send + 'static,
{
    h2: tower_h2::server::Connection<
        S,
        gen_server::NodeServer<NodeService<T>>,
        E,
        gen_server::node::ResponseBody<NodeService<T>>,
        (),
    >,
}

impl<S, T, E> Future for Connection<S, T, E>
where
    S: AsyncRead + AsyncWrite,
    T: Node + Clone + 'static,
    <T::BlockService as BlockService>::Header: Send + 'static,
    <T::ContentService as ContentService>::Message: Send + 'static,
    <T::GossipService as GossipService>::Node: Send + 'static,
    <T::GossipService as GossipService>::NodeId: Send + 'static,
    E: Executor<
        tower_h2::server::Background<
            gen_server::node::ResponseFuture<NodeService<T>>,
            gen_server::node::ResponseBody<NodeService<T>>,
        >,
    >,
{
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<(), Error> {
        self.h2.poll().map_err(|e| e.into())
    }
}

impl<T, E> Server<T, E>
where
    T: Node + Clone + 'static,
    <T::BlockService as BlockService>::Header: Send + 'static,
    <T::ContentService as ContentService>::Message: Send + 'static,
    <T::GossipService as GossipService>::Node: Send + 'static,
    <T::GossipService as GossipService>::NodeId: Send + 'static,
    E: Executor<
            tower_h2::server::Background<
                gen_server::node::ResponseFuture<NodeService<T>>,
                gen_server::node::ResponseBody<NodeService<T>>,
            >,
        > + Clone,
{
    /// Creates a server instance around the node service implementation.
    pub fn new(node: T, executor: E) -> Self {
        let grpc_service = gen_server::NodeServer::new(NodeService::new(node));
        let h2 = tower_h2::Server::new(grpc_service, Default::default(), executor);
        Server { h2 }
    }

    /// Initializes a client peer connection based on an accepted connection
    /// socket. The socket can be obtained from a stream returned by `listen`.
    pub fn serve<S>(&mut self, sock: S) -> Connection<S, T, E>
    where
        S: AsyncRead + AsyncWrite,
    {
        Connection {
            h2: self.h2.serve(sock),
        }
    }
}

/// Sets up a listening TCP socket bound to the given address.
/// If successful, returns an asynchronous stream of `TcpStream` socket
/// objects representing accepted TCP connections from clients.
/// The TCP_NODELAY option is disabled on the returned sockets as
/// necessary for the HTTP/2 protocol.
pub fn listen(
    addr: &SocketAddr,
) -> Result<impl Stream<Item = TcpStream, Error = tokio::io::Error>, tokio::io::Error> {
    let listener = TcpListener::bind(&addr)?;
    let stream = listener.incoming().and_then(|sock| {
        sock.set_nodelay(true)?;
        Ok(sock)
    });
    Ok(stream)
}

/// Sets up a listening Unix socket bound to the specified path.
/// If successful, returns an asynchronous stream of `UnixStream` socket
/// objects representing accepted connections from clients.
#[cfg(unix)]
pub fn listen_unix<P: AsRef<Path>>(
    path: P,
) -> Result<impl Stream<Item = UnixStream, Error = tokio::io::Error>, tokio::io::Error> {
    let listener = UnixListener::bind(path)?;
    Ok(listener.incoming())
}

/// The error type for gRPC server operations.
#[derive(Debug)]
pub enum Error {
    Http2Handshake(h2::Error),
    Http2Protocol(h2::Error),
    NewService(h2::Error),
    Service(h2::Error),
    Execute,
}

type H2Error<T> = tower_h2::server::Error<gen_server::NodeServer<NodeService<T>>>;

// Incorporating tower_h2::server::Error would be too cumbersome due to the
// type parameter baggage, see https://github.com/tower-rs/tower-h2/issues/50
// So we match it into our own variants.
impl<T> From<H2Error<T>> for Error
where
    T: Node + Clone,
    <T::BlockService as BlockService>::Header: Send + 'static,
    <T::ContentService as ContentService>::Message: Send + 'static,
    <T::GossipService as GossipService>::Node: Send + 'static,
    <T::GossipService as GossipService>::NodeId: Send + 'static,
{
    fn from(err: H2Error<T>) -> Self {
        use tower_h2::server::Error::*;
        match err {
            Handshake(e) => Error::Http2Handshake(e),
            Protocol(e) => Error::Http2Protocol(e),
            NewService(e) => Error::NewService(e),
            Service(e) => Error::Service(e),
            Execute => Error::Execute,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Http2Handshake(_) => write!(f, "HTTP/2 handshake error"),
            Error::Http2Protocol(_) => write!(f, "HTTP/2 protocol error"),
            Error::NewService(_) => write!(f, "service creation error"),
            Error::Service(_) => write!(f, "error returned by service"),
            Error::Execute => write!(f, "task execution error"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Http2Handshake(e) => Some(e),
            Error::Http2Protocol(e) => Some(e),
            Error::NewService(e) => Some(e),
            Error::Service(e) => Some(e),
            Error::Execute => None,
        }
    }
}
