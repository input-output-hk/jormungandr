use std::{net::SocketAddr, fmt, str, time::Duration};
#[cfg(unix)]
use std::path::{PathBuf};


/// configuration for the connection type.
/// Either to listen from, or to connect too.
///
/// On unix we also support `UnixSocket`. Otherwise the option
/// is not available.
///
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Connection {
    Tcp(SocketAddr),
    #[cfg(unix)]
    Unix(PathBuf),
}
impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Connection::Tcp(addr) => write!(f, "{}", addr),
            #[cfg(unix)]
            Connection::Unix(path)   => write!(f, "{}", path.to_string_lossy()),
        }
    }
}

/// Protocol to use for a connection.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    Ntt,
    Grpc,
}

const DEFAULT_TIMEOUT_MICROSECONDS : u64 = 500_000;

/// represent a connection peer
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peer {
    /// the connection to connect to
    pub connection: Connection,
    /// Network protocol to use for this connection.
    pub protocol: Protocol,
    /// a timeout in case of inactivity or timout between request.
    pub timeout:    Duration,
}

impl Peer {
    pub fn new(connection: Connection, protocol: Protocol) -> Self {
        Peer {
            connection,
            protocol,
            timeout: Duration::from_micros(DEFAULT_TIMEOUT_MICROSECONDS)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Listen {
    /// connection to listen to and start accepting connection from
    pub connection: Connection,
    /// Network protocol to use for this connection.
    pub protocol: Protocol,
    /// timeout of the connected peers. Will be set for when/if we
    /// send them commands, queries or else and they timedout.
    ///
    /// Every derived connection will receive this timeout
    pub timeout:    Duration,
}

impl Listen {
    pub fn new(connection: Connection, protocol: Protocol) -> Self {
        Listen {
            connection,
            protocol,
            timeout: Duration::from_micros(DEFAULT_TIMEOUT_MICROSECONDS)
        }
    }
}

/// The network static configuration settings
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Configuration {
    /// the node we will connect to.
    pub peer_nodes: Vec<Peer>,

    /// the different connection to listen to for new nodes
    /// to connect to our node
    pub listen_to: Vec<Listen>,
}
