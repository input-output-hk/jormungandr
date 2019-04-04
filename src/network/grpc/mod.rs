mod bootstrap;
mod client;
mod server;

use super::{p2p_topology as p2p, BlockConfig};
use crate::{
    blockcfg::{Block, BlockDate, Header, HeaderHash},
    blockchain::BlockchainR,
    settings::start::network::Peer,
};

use http::Uri;
use network_grpc::peer::TcpPeer;

use std::net::SocketAddr;

pub use self::client::run_connect_socket;
pub use self::server::run_listen_socket;

impl network_grpc::client::ProtocolConfig for BlockConfig {
    type Block = Block;
    type Header = Header;
    type BlockId = HeaderHash;
    type BlockDate = BlockDate;
    type Node = p2p::Node;
}

fn origin_uri(addr: SocketAddr) -> Uri {
    let authority = format!("{}:{}", addr.ip(), addr.port());
    http::uri::Builder::new()
        .scheme("http")
        .authority(authority.as_str())
        .path_and_query("/")
        .build()
        .unwrap()
}

pub fn bootstrap_from_peer(peer: Peer, blockchain: BlockchainR) {
    info!("connecting to bootstrap peer {}", peer.connection);
    let addr = peer.address();
    let origin = origin_uri(addr);
    let peer = TcpPeer::new(addr);
    bootstrap::bootstrap_from_target(peer, blockchain, origin)
}
