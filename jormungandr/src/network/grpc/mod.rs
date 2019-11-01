mod client;
mod server;

use crate::{
    blockcfg::{Block, BlockDate, Fragment, FragmentId, Header, HeaderHash},
    network::{
        p2p::{Gossip as NodeData, Id},
        BlockConfig,
    },
};

pub use self::client::{
    connect, fetch_block, ConnectError, ConnectFuture, Connection, FetchBlockError,
};
pub use self::server::run_listen_socket;

impl network_grpc::client::ProtocolConfig for BlockConfig {
    type Block = Block;
    type Header = Header;
    type BlockId = HeaderHash;
    type BlockDate = BlockDate;
    type Fragment = Fragment;
    type FragmentId = FragmentId;
    type Node = NodeData;
    type NodeId = Id;
}
