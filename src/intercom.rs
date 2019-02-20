use crate::blockcfg::BlockConfig;

use network_core::error::Code;

use futures::prelude::*;
use futures::sync::{mpsc, oneshot};

use std::{
    error,
    fmt::{self, Debug, Display},
    marker::PhantomData,
};

/// The error values passed via intercom messages.
#[derive(Debug)]
pub struct Error {
    code: Code,
    cause: Box<dyn error::Error + Send + Sync>,
}

impl Error {
    pub fn failed<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: Code::Failed,
            cause: cause.into(),
        }
    }

    pub fn unimplemented<S: Into<String>>(message: S) -> Self {
        Error {
            code: Code::Unimplemented,
            cause: message.into().into(),
        }
    }

    pub fn code(&self) -> Code {
        self.code
    }
}

impl From<oneshot::Canceled> for Error {
    fn from(src: oneshot::Canceled) -> Self {
        Error {
            code: Code::Canceled,
            cause: src.into(),
        }
    }
}

impl From<chain_storage::error::Error> for Error {
    fn from(err: chain_storage::error::Error) -> Self {
        use chain_storage::error::Error::*;

        let code = match err {
            BlockNotFound => Code::NotFound,
        };
        Error {
            code,
            cause: err.into(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.cause, f)
    }
}


impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&*self.cause)
    }
}

type ReplySender<T> = oneshot::Sender<Result<T, Error>>;

#[derive(Debug)]
pub struct ReplyHandle<T> {
    sender: ReplySender<T>,
}

impl<T> ReplyHandle<T> {
    pub fn reply(self, result: Result<T, Error>) {
        match self.sender.send(result) {
            Ok(()) => {}
            Err(_) => panic!("failed to send result"),
        }
    }

    pub fn reply_ok(self, response: T) {
        self.reply(Ok(response));
    }

    pub fn reply_error(self, error: Error) {
        self.reply(Err(error));
    }
}

pub struct ReplyFuture<T, E> {
    receiver: oneshot::Receiver<Result<T, Error>>,
    _phantom_error: PhantomData<E>,
}

impl<T, E> Future for ReplyFuture<T, E>
where
    E: From<Error>,
{
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Poll<T, E> {
        let item = match self.receiver.poll() {
            Err(oneshot::Canceled) => {
                warn!("response canceled by the client request task");
                return Err(Error::from(oneshot::Canceled).into());
            }
            Ok(Async::NotReady) => {
                return Ok(Async::NotReady);
            }
            Ok(Async::Ready(Err(e))) => {
                warn!("error processing request: {:?}", e);
                return Err(Error::from(e).into());
            }
            Ok(Async::Ready(Ok(item))) => item,
        };

        Ok(Async::Ready(item))
    }
}

pub fn unary_reply<T, E>() -> (ReplyHandle<T>, ReplyFuture<T, E>) {
    let (sender, receiver) = oneshot::channel();
    let future = ReplyFuture {
        receiver,
        _phantom_error: PhantomData,
    };
    (ReplyHandle { sender }, future)
}

#[derive(Debug)]
pub struct ReplyStreamHandle<T> {
    sender: mpsc::UnboundedSender<Result<T, Error>>,
}

impl<T> ReplyStreamHandle<T> {
    pub fn send(&mut self, item: T) {
        self.sender.unbounded_send(Ok(item)).unwrap()
    }

    pub fn send_error(&mut self, error: Error) {
        self.sender.unbounded_send(Err(error)).unwrap()
    }

    pub fn close(self) {
        self.sender.wait().close().unwrap();
    }
}

pub struct ReplyStream<T, E> {
    receiver: mpsc::UnboundedReceiver<Result<T, Error>>,
    _phantom_error: PhantomData<E>,
}

impl<T, E> Stream for ReplyStream<T, E>
where
    E: From<Error>,
{
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Poll<Option<T>, E> {
        match self.receiver.poll() {
            Err(()) => panic!("receiver returned an error"),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(Ok(item)))) => Ok(Async::Ready(Some(item))),
            Ok(Async::Ready(Some(Err(e)))) => {
                warn!("error while streaming response: {:?}", e);
                return Err(Error::from(e).into());
            }
        }
    }
}

pub fn stream_reply<T, E>() -> (ReplyStreamHandle<T>, ReplyStream<T, E>) {
    let (sender, receiver) = mpsc::unbounded();
    let stream = ReplyStream {
        receiver,
        _phantom_error: PhantomData,
    };
    (ReplyStreamHandle { sender }, stream)
}

/// ...
#[derive(Debug)]
pub enum TransactionMsg<B: BlockConfig> {
    ProposeTransaction(Vec<B::TransactionId>, ReplyHandle<Vec<bool>>),
    SendTransaction(Vec<B::Transaction>),
}

/// Client messages, mainly requests from connected peers to our node.
/// Fetching the block headers, the block, the tip
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
