mod bootstrap;
mod server;

use crate::{
    blockcfg::BlockConfig,
    blockchain::BlockchainR,
    settings::network::{Connection, Peer},
};

pub use self::server::run_listen_socket;

use chain_core::property;
use network_grpc::peer::TcpPeer;

#[cfg(unix)]
use network_grpc::peer::UnixPeer;

pub fn bootstrap_from_peer<B>(peer: Peer, blockchain: BlockchainR<B>)
where
    B: BlockConfig,
    <B::Ledger as property::Ledger>::Update: Clone,
    <B::Settings as property::Settings>::Update: Clone,
    <B::Leader as property::LeaderSelection>::Update: Clone,
    for<'a> &'a <B::Block as property::HasTransaction>::Transactions:
        IntoIterator<Item = &'a B::Transaction>,
{
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
