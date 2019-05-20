use futures::try_ready;
use tokio::net::tcp::{self, TcpStream};
#[cfg(unix)]
use tokio::net::unix::{self, UnixStream};
use tokio::prelude::*;
use tower_grpc::codegen::server::tower::Service;

use std::{io, net::SocketAddr};

#[cfg(unix)]
use std::path::Path;

/// A `MakeConnection` instance to establish TCP connections.
pub struct TcpConnector;

/// A `MakeConnection` instance to establish Unix socket connections.
///
/// This type is only available on Unix.
#[cfg(unix)]
pub struct UnixConnector;

impl Service<SocketAddr> for TcpConnector {
    type Response = TcpStream;
    type Error = io::Error;
    type Future = TcpConnectFuture;

    #[inline]
    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    #[inline]
    fn call(&mut self, addr: SocketAddr) -> Self::Future {
        TcpStream::connect(&addr).into()
    }
}

/// A future adapter that resolves to a `TcpStream` optimized for
/// the HTTP/2 protocol.
/// It attempts to set the socket option `TCP_NODELAY` to true before
/// resolving with the connection. Failure to set the option is silently
/// ignored, which may result in degraded latency.
pub struct TcpConnectFuture {
    inner: tcp::ConnectFuture,
}

impl From<tcp::ConnectFuture> for TcpConnectFuture {
    #[inline]
    fn from(src: tcp::ConnectFuture) -> Self {
        TcpConnectFuture { inner: src }
    }
}

impl Future for TcpConnectFuture {
    type Item = TcpStream;
    type Error = io::Error;
    fn poll(&mut self) -> Result<Async<TcpStream>, io::Error> {
        let stream = try_ready!(self.inner.poll());
        stream.set_nodelay(true).unwrap_or(());
        Ok(stream.into())
    }
}

#[cfg(unix)]
impl<P> Service<P> for UnixConnector
where
    P: AsRef<Path>,
{
    type Response = UnixStream;
    type Error = io::Error;
    type Future = unix::ConnectFuture;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, path: P) -> Self::Future {
        UnixStream::connect(path)
    }
}
