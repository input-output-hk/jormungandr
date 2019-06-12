mod bootstrap;
mod client;
mod server;

use super::{p2p::topology as p2p, BlockConfig};
use crate::blockcfg::{Block, BlockDate, Header, HeaderHash};

pub use self::bootstrap::bootstrap_from_peer;
pub use self::client::{connect, fetch_block, Connection};
pub use self::server::run_listen_socket;

impl network_grpc::client::ProtocolConfig for BlockConfig {
    type Block = Block;
    type Header = Header;
    type BlockId = HeaderHash;
    type BlockDate = BlockDate;
    type Node = p2p::Node;
    type NodeId = p2p::NodeId;
}
