mod bootstrap;
mod client;
mod server;

use super::{p2p_topology as p2p, BlockConfig};
use crate::{
    blockcfg::{Block, BlockDate, Header, HeaderHash},
    blockchain::BlockchainR,
    settings::start::network::Peer,
};

use http::{uri, HttpTryFrom};
use network_grpc::peer::TcpPeer;

use std::net::SocketAddr;

pub use self::client::connect;
pub use self::server::run_listen_socket;

impl network_grpc::client::ProtocolConfig for BlockConfig {
    type Block = Block;
    type Header = Header;
    type BlockId = HeaderHash;
    type BlockDate = BlockDate;
    type Node = p2p::Node;
}

fn origin_authority(addr: SocketAddr) -> uri::Authority {
    let authority = format!("{}:{}", addr.ip(), addr.port());
    HttpTryFrom::try_from(authority.as_str()).unwrap()
}

pub fn bootstrap_from_peer(peer: Peer, blockchain: BlockchainR) {
    info!("connecting to bootstrap peer {}", peer.connection);
    let addr = peer.address();
    let origin = origin_authority(addr);
    let peer = TcpPeer::new(addr);
    bootstrap::bootstrap_from_target(peer, blockchain, origin)
}
