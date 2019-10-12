mod client;
mod server;

use super::{p2p::topology as p2p, BlockConfig};
use crate::blockcfg::{Block, BlockDate, Fragment, FragmentId, Header, HeaderHash};

pub use self::client::{connect, fetch_block, Connection, FetchBlockError};
pub use self::server::run_listen_socket;

impl network_grpc::client::ProtocolConfig for BlockConfig {
    type Block = Block;
    type Header = Header;
    type BlockId = HeaderHash;
    type BlockDate = BlockDate;
    type Fragment = Fragment;
    type FragmentId = FragmentId;
    type Node = p2p::NodeData;
    type NodeId = p2p::NodeId;
}
