use std::{net::SocketAddr, path::PathBuf, fmt, str, time::Duration};

/// configuration for the connection type.
/// Either to listen from, or to connect too.
///
/// On unix we also support `UnixSocket`. Otherwise the option
/// is not available.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Connection {
    Socket(SocketAddr),
    #[cfg(unix)]
    Unix(PathBuf),
}
impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Connection::Socket(addr) => write!(f, "{}", addr),
            #[cfg(unix)]
            Connection::Unix(path)   => write!(f, "{:?}", path),
        }
    }
}

impl str::FromStr for Connection {
    type Err = <SocketAddr as str::FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let r = s.parse().map(Connection::Socket);

        match r {
            Ok(sock) => Ok(sock),
            #[cfg(unix)]
            Err(_)   => Ok(Connection::Unix(s.into())),
            #[cfg(not(unix))]
            Err(err) => Err(err),
        }

    }
}

const DEFAULT_TIMEOUT_MICROSECONDS : u64 = 500_000;

/// represent a connection peer
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peer {
    /// the connection to connect to
    pub connection: Connection,
    /// a timeout in case of inactivity or timout between request.
    pub timeout:    Duration,
}
impl str::FromStr for Peer {
    type Err = <Connection as str::FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(|connection| {
            Peer {
                connection: connection,
                timeout:    Duration::from_micros(DEFAULT_TIMEOUT_MICROSECONDS),
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Listen {
    /// connection to listen to and start accepting connection from
    pub connection: Connection,
    /// timeout of the connected peers. Will be set for when/if we
    /// send them commands, queries or else and they timedout.
    ///
    /// Every derived connection will receive this timeout
    pub timeout:    Duration,
}
impl str::FromStr for Listen {
    type Err = <Connection as str::FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(|connection| {
            Listen {
                connection: connection,
                timeout:    Duration::from_micros(DEFAULT_TIMEOUT_MICROSECONDS),
            }
        })
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
