mod bootstrap;
mod client;
mod server;

use super::NetworkBlockConfig;
use crate::{blockchain::BlockchainR, settings::start::network::Peer};

pub use self::client::run_connect_socket;
pub use self::server::run_listen_socket;

use bytes::Bytes;
use http;
use network_grpc::peer::TcpPeer;

pub fn bootstrap_from_peer<B>(peer: Peer, blockchain: BlockchainR<B>)
where
    B: NetworkBlockConfig,
{
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
