use crate::blockcfg::BlockConfig;

use futures::prelude::*;
use futures::sync::{mpsc, oneshot};

use std::fmt::{self, Debug, Display};

/// The error values passed via intercom messages.
#[derive(Debug)]
pub struct Error(Box<dyn std::error::Error + Send + Sync>);

impl Error {
    pub fn from_error<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Error(error.into())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Error {
        Error(s.into())
    }
}

impl<'a> From<&'a str> for Error {
    fn from(s: &'a str) -> Error {
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

type ReplySender<T> = oneshot::Sender<Result<T, Error>>;

#[derive(Debug)]
pub struct ReplyHandle<T> {
    sender: ReplySender<T>,
}

impl<T> ReplyHandle<T> {
    pub fn reply_ok(self, response: T) {
        self.sender.send(Ok(response)).unwrap();
    }

    pub fn reply_error(self, error: Error) {
        self.sender.send(Err(error)).unwrap();
    }
}

pub struct ReplyFuture<T> {
    receiver: oneshot::Receiver<Result<T, Error>>,
}

impl<T> Future for ReplyFuture<T> {
    type Item = T;
    type Error = Error;

    fn poll(&mut self) -> Poll<T, Error> {
        let item = match self.receiver.poll() {
            Err(oneshot::Canceled) => {
                warn!("response canceled by the client request task");
                // FIXME: a non-stringized error code needed here
                return Err(Error::from("canceled"));
            }
            Ok(Async::NotReady) => {
                return Ok(Async::NotReady);
            }
            Ok(Async::Ready(Err(e))) => {
                warn!("error processing request: {:?}", e);
                return Err(Error::from_error(e));
            }
            Ok(Async::Ready(Ok(item))) => item,
        };

        Ok(Async::Ready(item))
    }
}

pub fn unary_reply<T>() -> (ReplyHandle<T>, ReplyFuture<T>) {
    let (sender, receiver) = oneshot::channel();
    (ReplyHandle { sender }, ReplyFuture { receiver })
}

#[derive(Debug)]
pub struct ReplyStreamHandle<T> {
    sender: mpsc::UnboundedSender<Result<T, Error>>,
}

impl<T> ReplyStreamHandle<T> {
    fn send(&mut self, item: T) {
        self.sender.unbounded_send(Ok(item)).unwrap()
    }

    fn send_error(&mut self, error: Error) {
        self.sender.unbounded_send(Err(error)).unwrap()
    }

    fn close(&mut self) {
        self.sender.close().unwrap();
    }
}

pub struct ReplyStream<T> {
    receiver: mpsc::UnboundedReceiver<Result<T, Error>>,
}

impl<T> Stream for ReplyStream<T> {
    type Item = T;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<T>, Error> {
        match try_ready!(self.receiver.poll()) {
            None => Ok(Async::Ready(None)),
            Some(Ok(item)) => Ok(Async::Ready(Some(item))),
            Some(Err(e)) => {
                warn!("error while streaming response: {:?}", e);
                return Err(Error::from_error(e));
            }
        }
    }
}

pub fn stream_reply<T>() -> (ReplyStreamHandle<T>, ReplyStream<T>) {
    let (sender, receiver) = mpsc::unbounded();
    (ReplyStreamHandle { sender }, ReplyStream { receiver })
}

/// ...
#[derive(Debug)]
pub enum TransactionMsg<B: BlockConfig> {
    ProposeTransaction(Vec<B::TransactionId>, ReplyHandle<Vec<bool>>),
    SendTransaction(Vec<B::Transaction>),
}

/// Client messages, mainly requests from connected peers to our node.
/// Fetching the block headers, the block, the tip
#[derive(Debug)]
pub enum ClientMsg<B: BlockConfig> {
    GetBlockTip(ReplyHandle<B::BlockHeader>),
    GetBlockHeaders(
        Vec<B::BlockHash>,
        B::BlockHash,
        ReplyHandle<Vec<B::BlockHeader>>,
    ),
    GetBlocks(B::BlockHash, B::BlockHash, ReplyStreamHandle<B::Block>),
    PullBlocksToTip(Vec<B::BlockHash>, ReplyStreamHandle<B::Block>),
}

impl<B> Debug for ClientMsg<B>
where
    B: BlockConfig,
    B::BlockHash: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientMsg::GetBlockTip(_) => f
                .debug_tuple("GetBlockTip")
                .field(&format_args!("_"))
                .finish(),
            ClientMsg::GetBlockHeaders(from, to, _) => f
                .debug_tuple("GetBlockHeaders")
                .field(from)
                .field(to)
                .field(&format_args!("_"))
                .finish(),
            ClientMsg::GetBlocks(from, to, _) => f
                .debug_tuple("GetBlocks")
                .field(from)
                .field(to)
                .field(&format_args!("_"))
                .finish(),
            ClientMsg::PullBlocksToTip(from, _) => f
                .debug_tuple("PullBlocksToTip")
                .field(from)
                .field(&format_args!("_"))
                .finish(),
        }
    }
}

/// General Block Message for the block task
pub enum BlockMsg<B: BlockConfig> {
    /// A untrusted Block has been received from the network task
    NetworkBlock(B::Block),
    /// A trusted Block has been received from the leadership task
    LeadershipBlock(B::Block),
}

impl<B> Debug for BlockMsg<B>
where
    B: BlockConfig,
    B::Block: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use BlockMsg::*;
        match self {
            NetworkBlock(block) => f.debug_tuple("NetworkBlock").field(block).finish(),
            LeadershipBlock(block) => f.debug_tuple("LeadershipBlock").field(block).finish(),
        }
    }
}

/// Message to broadcast to all the connected peers (that requested to subscribe
/// to our blockchain).
///
pub enum NetworkBroadcastMsg<B: BlockConfig> {
    Block(B::Block),
    Header(B::BlockHeader),
    Transaction(B::Transaction),
}

impl<B> Debug for NetworkBroadcastMsg<B>
where
    B: BlockConfig,
    B::Block: Debug,
    B::BlockHeader: Debug,
    B::Transaction: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use NetworkBroadcastMsg::*;
        match self {
            Block(block) => f.debug_tuple("Block").field(block).finish(),
            Header(header) => f.debug_tuple("Header").field(header).finish(),
            Transaction(tx) => f.debug_tuple("Transaction").field(tx).finish(),
        }
    }
}
