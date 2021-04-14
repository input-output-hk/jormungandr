/// This module is responsible for handling active peers and communication in a p2p setting.
/// It takes care of managing connections with said peers and sending messages to them.
/// The topology task is instead responsible for the discovery of active peers.
///
/// FIXME: Topology and peers have indipendent representation of a external node.
/// At the moment, topology and peers each use a different ID for node identification
/// but since those are not checked, each layers ignore the other one's IDs.
/// Ideally, peers should request a proof-of-possession of the key used to
/// authenticate gossips when connecting and viceversa.
/// Leaving this to when we will introduce identity verification, since requirements
/// are likely to change.
pub mod comm;

pub type Address = std::net::SocketAddr;
