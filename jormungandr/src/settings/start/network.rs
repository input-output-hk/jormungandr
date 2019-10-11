use crate::network::p2p::topology::NodeId;
use crate::settings::start::config::{Address, InterestLevel, Topic};
use poldercast::PrivateId;
use std::{collections::BTreeMap, net::SocketAddr, str, time::Duration};

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
    /// Optional public IP address to advertise.
    /// Also used as the binding address unless the `listen` field
    /// is set with an address value.
    pub public_address: Option<Address>,

    pub private_id: PrivateId,

    /// Local socket address to listen to, if different from public address.
    /// The IP address can be given as 0.0.0.0 or :: to bind to all
    /// network interfaces.
    pub listen_address: Option<SocketAddr>,

    /// list of trusted addresses
    pub trusted_peers: Vec<poldercast::Address>,

    /// the protocol to utilise for the p2p network
    pub protocol: Protocol,

    /// the topic we are interested to hear about
    pub subscriptions: BTreeMap<Topic, InterestLevel>,

    /// Maximum allowed number of peer connections.
    pub max_connections: usize,

    /// the default value for the timeout for inactive connection
    pub timeout: Duration,

    /// Whether to allow non-public IP addresses in gossip
    pub allow_private_addresses: bool,
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
    pub fn private_id(&self) -> &PrivateId {
        &self.private_id
    }

    pub fn public_id(&self) -> NodeId {
        NodeId(self.private_id().id())
    }

    /// Returns the listener configuration, if the options defining it
    /// were set.
    pub fn listen(&self) -> Option<Listen> {
        self.listen_address
            .or(self
                .public_address
                .as_ref()
                .and_then(|address| address.to_socketaddr()))
            .map(|addr| Listen::new(addr, self.protocol))
    }
}
