mod bootstrap;
mod server;

use crate::{
    blockcfg::BlockConfig,
    blockchain::BlockchainR,
    settings::network::{Connection, Peer},
};

pub use self::server::run_listen_socket;

use network_grpc::peer::TcpPeer;

#[cfg(unix)]
use network_grpc::peer::UnixPeer;

pub fn bootstrap_from_peer<B: BlockConfig>(peer: Peer, blockchain: BlockchainR<B>) {
    info!("connecting to bootstrap peer {}", peer.connection);
    match peer.connection {
        Connection::Tcp(addr) => {
            let peer = TcpPeer::new(addr);
            bootstrap::bootstrap_from_target(peer, blockchain)
        }
        #[cfg(unix)]
        Connection::Unix(path) => {
            let peer = UnixPeer::new(path);
            bootstrap::bootstrap_from_target(peer, blockchain)
        }
    }
}
