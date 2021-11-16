/// This module is responsible for handling active peers and communication in a p2p setting.
/// It takes care of managing connections with said peers and sending messages to them.
/// The topology task is instead responsible for the discovery of active peers.
pub mod comm;

/// At the logical level, every peer is identified by its public key, and this is the only
/// info exposed in the external interface.
/// However, keeping the remote address of the peer is needed for some features, namely
/// debugging and server side authentication of requests.
/// In addition, keep in mind the address here is not always the public address included in the
/// gossip, as it may be an ephemeral address used by a connected client.
pub type Address = std::net::SocketAddr;
