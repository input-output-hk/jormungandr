use super::config;
use crate::{
    network::p2p::Address,
    topology::{layers::LayersConfig, NodeId, QuarantineConfig},
};
use chain_crypto::Ed25519;
use jormungandr_lib::{crypto::key::SigningKey, multiaddr};
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

/// The limit on the number of simultaneous P2P client connections
/// used unless the corresponding configuration option is specified.
pub const DEFAULT_MAX_CLIENT_CONNECTIONS: usize = 192;

/// The default timeout for connections
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

///
/// The network static configuration settings
#[derive(Clone)]
pub struct Configuration {
    /// Local socket address to listen to, if different from public address.
    /// The IP address can be given as 0.0.0.0 or :: to bind to all
    /// network interfaces.
    pub listen_address: Option<SocketAddr>,

    pub public_address: Option<Address>,

    // Secret key used to authenticate communications, the public part is used as an identifier of the node
    pub node_key: SigningKey<Ed25519>,

    /// list of trusted addresses
    pub trusted_peers: Vec<TrustedPeer>,

    /// the protocol to utilise for the p2p network
    pub protocol: Protocol,

    /// Maximum allowed number of peer connections.
    pub max_connections: usize,

    /// Maximum allowed number of client connections.
    pub max_client_connections: usize,

    /// the default value for the timeout for inactive connection
    pub timeout: Duration,

    pub policy: QuarantineConfig,

    pub layers: LayersConfig,

    /// Whether to allow non-public IP addresses in gossip
    pub allow_private_addresses: bool,

    pub gossip_interval: Duration,

    pub network_stuck_check: Duration,

    pub max_bootstrap_attempts: Option<usize>,

    /// Whether to limit bootstrap to trusted peers (which increase their load / reduce their connectivities)
    pub bootstrap_from_trusted_peers: bool,

    /// Whether to skip bootstrap, not recommended in normal settings. useful to true for self-node
    pub skip_bootstrap: bool,

    pub http_fetch_block0_service: Vec<String>,
}

/// Trusted peer with DNS address resolved.
#[derive(Clone, Hash)]
pub struct TrustedPeer {
    pub addr: SocketAddr,
    // This will need to become compulsory if we want to check validity of keys/ids
    pub id: Option<NodeId>,
}

#[derive(Debug, thiserror::Error)]
pub enum PeerResolveError {
    #[error("DNS address resolution failed")]
    Resolve(#[from] multiaddr::Error),
    #[error("Address shall consist of a host address and a TCP component")]
    InvalidAddress,
}

impl TrustedPeer {
    pub fn resolve(peer: &config::TrustedPeer) -> Result<Self, PeerResolveError> {
        let addr = multiaddr::to_tcp_socket_addr(&multiaddr::resolve_dns(&peer.address)?)
            .ok_or(PeerResolveError::InvalidAddress)?;
        Ok(TrustedPeer {
            addr,
            id: peer.id.clone().map(Into::into),
        })
    }
}

impl Peer {
    pub fn new(connection: SocketAddr) -> Self {
        Peer::with_timeout(connection, DEFAULT_TIMEOUT)
    }

    pub fn with_timeout(connection: SocketAddr, timeout: Duration) -> Self {
        Peer {
            connection,
            protocol: Protocol::Grpc,
            timeout,
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.connection
    }
}

impl Listen {
    pub fn new(connection: SocketAddr) -> Self {
        Listen {
            connection,
            protocol: Protocol::Grpc,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.connection
    }
}

impl Configuration {
    pub fn address(&self) -> Option<Address> {
        self.public_address
    }

    /// Returns the listener configuration, if the options defining it
    /// were set.
    pub fn listen(&self) -> Option<Listen> {
        self.listen_address.or(self.public_address).map(Listen::new)
    }
}
