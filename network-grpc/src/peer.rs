use tokio::net::tcp::{self, TcpStream};
#[cfg(unix)]
use tokio::net::unix::{self, UnixStream};

use std::{io, net::SocketAddr};

#[cfg(unix)]
use std::path::{Path, PathBuf};

/// Specifies the connection details of a remote TCP/IP peer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TcpPeer {
    addr: SocketAddr,
}

impl TcpPeer {
    pub fn new(addr: SocketAddr) -> Self {
        TcpPeer { addr }
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }
}

/// Specifies the connection details of a local Unix socket peer.
///
/// This type is only available on Unix.
#[cfg(unix)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnixPeer {
    path: PathBuf,
}

#[cfg(unix)]
impl UnixPeer {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        UnixPeer { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl tokio_connect::Connect for TcpPeer {
    type Connected = TcpStream;
    type Error = io::Error;
    type Future = tcp::ConnectFuture;

    fn connect(&self) -> tcp::ConnectFuture {
        TcpStream::connect(self.addr())
    }
}

#[cfg(unix)]
impl tokio_connect::Connect for UnixPeer {
    type Connected = UnixStream;
    type Error = io::Error;
    type Future = unix::ConnectFuture;

    fn connect(&self) -> unix::ConnectFuture {
        UnixStream::connect(self.path())
    }
}
