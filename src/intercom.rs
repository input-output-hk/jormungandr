use blockcfg::{Block, Header, BlockHash, Transaction};
use protocol::{protocol, network_transport::LightWeightConnectionId};
use futures::sync::mpsc::UnboundedSender;

use std::fmt::{self, Debug, Display};

/// The error values passed via intercom messages.
#[derive(Debug)]
pub struct Error(Box<dyn std::error::Error + Send + Sync>);

impl Error {
    pub fn from_error<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static
    {
        Error(error.into())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Error {
        Error(s.into())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&std::error::Error> {
        self.0.cause()
    }
}

pub trait Reply<T>: Debug {
    fn reply_ok(&self, item: T);
    fn reply_error(&self, error: Error);

    fn reply(&self, result: Result<T, Error>) {
        match result {
            Ok(item) => self.reply_ok(item),
            Err(error) => self.reply_error(error),
        }
    }
}

pub trait StreamReply<T>: Debug {
    fn send(&self, item: T);
    fn send_error(&self, error: Error);
    fn close(&self);
}

pub type BoxReply<T> = Box<dyn Reply<T> + Send>;
pub type BoxStreamReply<T> = Box<dyn StreamReply<T> + Send>;

/// Simple RAII for the reply information to NTT protocol commands
#[derive(Clone, Debug)]
pub struct NttReplyHandle {
    // the identifier of the connection we are replying to
    identifier: LightWeightConnectionId,
    // the appropriate sink to send the messages to
    sink: UnboundedSender<protocol::Message>,
}

impl NttReplyHandle {
    pub fn new(
        identifier: LightWeightConnectionId,
        sink: UnboundedSender<protocol::Message>,
    ) -> Self {
        NttReplyHandle { identifier, sink }
    }

    fn send_message(&self, message: protocol::Message) {
        self.sink.unbounded_send(message).unwrap();
    }

    fn send_close(&self) {
        self.sink.unbounded_send(
            protocol::Message::CloseConnection(self.identifier)
        ).unwrap();
    }
}

impl Reply<Vec<Header>> for NttReplyHandle {
    fn reply_ok(&self, item: Vec<Header>) {
        self.send_message(
            protocol::Message::BlockHeaders(
                self.identifier,
                protocol::Response::Ok(item.into()),
            )
        );
        self.send_close();
    }

    fn reply_error(&self, error: Error) {
        self.send_message(
            protocol::Message::BlockHeaders(
                self.identifier,
                protocol::Response::Err(error.to_string()),
            )
        );
        self.send_close();
    }
}

impl Reply<Header> for NttReplyHandle {
    fn reply_ok(&self, item: Header) {
        self.send_message(
            protocol::Message::BlockHeaders(
                self.identifier,
                protocol::Response::Ok(protocol::BlockHeaders(vec![item])),
            )
        );
        self.send_close();
    }

    fn reply_error(&self, error: Error) {
        self.send_message(
            protocol::Message::BlockHeaders(
                self.identifier,
                protocol::Response::Err(error.to_string()),
            )
        );
        self.send_close();
    }
}

impl StreamReply<Block> for NttReplyHandle {
    fn send(&self, blk: Block) {
        self.send_message(
            protocol::Message::Block(
                self.identifier,
                protocol::Response::Ok(blk),
            )
        );
    }

    fn send_error(&self, error: Error) {
        self.send_message(
            protocol::Message::Block(
                self.identifier,
                protocol::Response::Err(error.to_string()),
            )
        );
    }

    fn close(&self) {
        self.send_close()
    }
}

// TODO

pub type TransactionMsg = u32;

/// Client messages, mainly requests from connected peers to our node.
/// Fetching the block headers, the block, the tip
#[derive(Debug)]
pub enum ClientMsg {
    GetBlockTip(BoxReply<Header>),
    GetBlockHeaders(Vec<BlockHash>, BlockHash, BoxReply<Vec<Header>>),
    GetBlocks(BlockHash, BlockHash, BoxStreamReply<Block>),
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
