mod bootstrap;
mod client;
mod server;

use super::{p2p_topology as p2p, BlockConfig};
use crate::blockcfg::{Block, BlockDate, Header, HeaderHash};

use http::{uri, HttpTryFrom};

use std::net::SocketAddr;

pub use self::bootstrap::bootstrap_from_peer;
pub use self::client::{connect, fetch_block};
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
