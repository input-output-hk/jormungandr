mod bootstrap;
mod client;
mod server;

use super::{p2p_topology as p2p, BlockConfig};
use crate::{
    blockcfg::{Block, BlockDate, Header, HeaderHash},
    blockchain::BlockchainR,
    settings::start::network::Peer,
};

pub use self::client::run_connect_socket;
pub use self::server::run_listen_socket;

use bytes::Bytes;
use http;
use network_grpc::peer::TcpPeer;

impl network_grpc::client::ProtocolConfig for BlockConfig {
    type Block = Block;
    type Header = Header;
    type BlockId = HeaderHash;
    type BlockDate = BlockDate;
    type Node = p2p::Node;
}

pub fn bootstrap_from_peer(peer: Peer, blockchain: BlockchainR) {
    info!("connecting to bootstrap peer {}", peer.connection);
    let authority = http::uri::Authority::from_shared(Bytes::from(format!(
        "{}:{}",
        peer.address().ip(),
        peer.address().port()
    )))
    .unwrap();
    let origin = http::uri::Builder::new()
        .scheme("http")
        .authority(authority)
        .path_and_query("/")
        .build()
        .unwrap();
    let peer = TcpPeer::new(*peer.address());
    bootstrap::bootstrap_from_target(peer, blockchain, origin)
}
