use blockcfg::{Block, Header, BlockHash, Transaction};
use std::{marker::{PhantomData}};
use protocol::{protocol, network_transport::LightWeightConnectionId};
use futures::{self, Stream, Future};

/// Simple RAII for the Reply to network commands
#[derive(Clone, Debug)]
pub struct NetworkHandler<A> {
    /// the identifier of the connection we are replying to
    pub identifier: LightWeightConnectionId,
    /// the appropriate sink to send the messages to
    pub sink: futures::sync::mpsc::UnboundedSender<protocol::Message>,
    /// marker for the type we are sending
    pub marker: PhantomData<A>,
}
pub trait Reply: Sized {
    type Item;
    type Error;
    fn reply_ok(&self, handler: &NetworkHandler<Self>, item: Self::Item);
    fn reply_error(&self, handler: &NetworkHandler<Self>, item: Self::Error);
}

#[derive(Clone, Debug)]
pub struct ClientMsgGetBlocks;
impl Reply for ClientMsgGetBlocks {
    type Item = Vec<Block>;
    type Error = ();
    fn reply_ok(&self, handler: &NetworkHandler<Self>, item: Self::Item) {
        futures::stream::iter_ok::<_, futures::sync::mpsc::SendError<protocol::Message>>(item)
            .map(|blk| protocol::Message::Block(handler.identifier, protocol::Response::Ok(blk)))
            .forward(&handler.sink).wait().unwrap();
    }
    fn reply_error(&self, handler: &NetworkHandler<Self>, item: Self::Error) {
        unimplemented!()
    }
}

#[derive(Clone, Debug)]
pub struct ClientMsgGetHeaders;
impl Reply for ClientMsgGetHeaders {
    type Item = Vec<Header>;
    type Error = ();
    fn reply_ok(&self, handler: &NetworkHandler<Self>, item: Self::Item) {
        handler.sink.unbounded_send(
            protocol::Message::BlockHeaders(
                handler.identifier,
                protocol::Response::Ok(item.into())
            )
        ).unwrap()
    }
    fn reply_error(&self, handler: &NetworkHandler<Self>, item: Self::Error) {
        unimplemented!()
    }
}

// TODO

pub type TransactionMsg = u32;

/// Client messages, mainly requests from connected peers to our node.
/// Fetching the block headers, the block, the tip
#[derive(Debug, Clone)]
pub enum ClientMsg {
    GetBlockTip(NetworkHandler<ClientMsgGetHeaders>),
    GetBlockHeaders(Vec<BlockHash>, BlockHash, NetworkHandler<ClientMsgGetHeaders>),
    GetBlocks(BlockHash, BlockHash, NetworkHandler<ClientMsgGetBlocks>),
}

/// General Block Message for the block task
#[derive(Debug, Clone)]
pub enum BlockMsg {
    /// A untrusted Block has been received from the network task
    NetworkBlock(Block),
    /// A trusted Block has been received from the leadership task
    LeadershipBlock(Block),
}

/// Message to broadcast to all the connected peers (that requested to subscribe
/// to our blockchain).
///
#[derive(Debug, Clone)]
pub enum NetworkBroadcastMsg {
    Block(Block),
    Header(Header),
    Transaction(Transaction),
}
