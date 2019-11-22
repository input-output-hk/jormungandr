use crate::network::p2p::{Id, PolicyConfig};
use poldercast::NodeProfile;
use std::{net::SocketAddr, str, time::Duration};

/// Protocol to use for a connection.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    Ntt,
    Grpc,
}

/// represent a connection peer
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peer {
    /// the connection to connect to
    pub connection: SocketAddr,
    /// Network protocol to use for this connection.
    pub protocol: Protocol,
    /// a timeout in case of inactivity or timout between request.
    pub timeout: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Listen {
    /// connection to listen to and start accepting connection from
    pub connection: SocketAddr,
    /// Network protocol to use for this connection.
    pub protocol: Protocol,
    /// timeout of the connected peers. Will be set for when/if we
    /// send them commands, queries or else and they timedout.
    ///
    /// Every derived connection will receive this timeout
    pub timeout: Duration,
}

/// The limit on the number of simultaneous P2P connections
/// used unless the corresponding configuration option is specified.
pub const DEFAULT_MAX_CONNECTIONS: usize = 256;

const DEFAULT_TIMEOUT_MICROSECONDS: u64 = 500_000;

///
/// The network static configuration settings
#[derive(Clone)]
pub struct Configuration {
    /// Local socket address to listen to, if different from public address.
    /// The IP address can be given as 0.0.0.0 or :: to bind to all
    /// network interfaces.
    pub listen_address: Option<SocketAddr>,

    pub profile: NodeProfile,

    /// list of trusted addresses
    pub trusted_peers: Vec<TrustedPeer>,

    /// the protocol to utilise for the p2p network
    pub protocol: Protocol,

    /// Maximum allowed number of peer connections.
    pub max_connections: usize,

    /// the default value for the timeout for inactive connection
    pub timeout: Duration,

    pub policy: PolicyConfig,

    /// Whether to allow non-public IP addresses in gossip
    pub allow_private_addresses: bool,

    pub max_unreachable_nodes_to_connect_per_event: Option<usize>,

    pub gossip_interval: Duration,

    pub topology_force_reset_interval: Option<Duration>,
}

#[derive(Clone)]
pub struct TrustedPeer {
    pub address: poldercast::Address,
    pub id: Id,
}

impl From<super::config::TrustedPeer> for TrustedPeer {
    fn from(tp: super::config::TrustedPeer) -> Self {
        TrustedPeer {
            address: tp.address.0,
            id: tp.id,
        }
    }
}

impl Peer {
    pub fn new(connection: SocketAddr, protocol: Protocol) -> Self {
        Peer {
            connection,
            protocol,
            timeout: Duration::from_micros(DEFAULT_TIMEOUT_MICROSECONDS),
        }
    }
    pub fn address(&self) -> SocketAddr {
        self.connection
    }
}

impl Listen {
    pub fn new(connection: SocketAddr, protocol: Protocol) -> Self {
        Listen {
            connection,
            protocol,
            timeout: Duration::from_micros(DEFAULT_TIMEOUT_MICROSECONDS),
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.connection
    }
}

impl Configuration {
    pub fn public_id(&self) -> Id {
        (*self.profile.id()).into()
    }

    /// Returns the listener configuration, if the options defining it
    /// were set.
    pub fn listen(&self) -> Option<Listen> {
        self.listen_address
            .or(self
                .profile
                .address()
                .and_then(|address| address.to_socketaddr()))
            .map(|addr| Listen::new(addr, self.protocol))
    }
}
