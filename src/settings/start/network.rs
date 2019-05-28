use std::{collections::BTreeMap, net::SocketAddr, str, time::Duration};

use crate::{
    network::p2p::topology::NodeId,
    settings::start::config::{Address, InterestLevel, Topic, TrustedPeer},
};

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

const DEFAULT_TIMEOUT_MICROSECONDS: u64 = 500_000;

///
/// The network static configuration settings
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configuration {
    /// Optional Node identifier. If not specified, a random identifier
    /// is generated.
    pub public_id: Option<NodeId>,

    /// Optional public IP address to advertise.
    /// Also used as the binding address unless the `listen_address` field
    /// is set with an address value.
    pub public_address: Option<Address>,

    /// Local socket address to listen to, if different from public address.
    /// The IP address can be given as 0.0.0.0 or :: to bind to all
    /// network interfaces.
    pub listen: Option<SocketAddr>,

    /// list of trusted addresses
    pub trusted_peers: Vec<TrustedPeer>,

    /// the protocol to utilise for the p2p network
    pub protocol: Protocol,

    /// the topic we are interested to hear about
    pub subscriptions: BTreeMap<Topic, InterestLevel>,

    /// the default value for the timeout for inactive connection
    pub timeout: Duration,
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
    /// Returns the listener configuration, if the options defining it
    /// were set.
    pub fn listen(&self) -> Option<Listen> {
        self.listen
            .or(self
                .public_address
                .as_ref()
                .and_then(|address| address.to_socketaddr()))
            .map(|addr| Listen::new(addr, self.protocol))
    }
}
