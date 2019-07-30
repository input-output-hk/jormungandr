use crate::blockcfg::{Block, Epoch, Fragment, FragmentId, Header, HeaderHash};
use crate::network::p2p::topology::NodeId;
use futures::prelude::*;
use futures::sync::{mpsc, oneshot};
use jormungandr_lib::interfaces::FragmentOrigin;
use network_core::error as core_error;
use slog::Logger;
use std::{
    error,
    fmt::{self, Debug, Display},
    marker::PhantomData,
};

/// The error values passed via intercom messages.
#[derive(Debug)]
pub struct Error {
    code: core_error::Code,
    cause: Box<dyn error::Error + Send + Sync>,
}

impl Error {
    pub fn failed<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: core_error::Code::Internal,
            cause: cause.into(),
        }
    }

    pub fn failed_precondition<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: core_error::Code::FailedPrecondition,
            cause: cause.into(),
        }
    }

    pub fn not_found<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: core_error::Code::NotFound,
            cause: cause.into(),
        }
    }

    pub fn unimplemented<S: Into<String>>(message: S) -> Self {
        Error {
            code: core_error::Code::Unimplemented,
            cause: message.into().into(),
        }
    }

    pub fn code(&self) -> core_error::Code {
        self.code
    }
}

impl From<oneshot::Canceled> for Error {
    fn from(src: oneshot::Canceled) -> Self {
        Error {
            code: core_error::Code::Unavailable,
            cause: src.into(),
        }
    }
}

impl From<chain_storage::error::Error> for Error {
    fn from(err: chain_storage::error::Error) -> Self {
        use chain_storage::error::Error::*;

        let code = match err {
            BlockNotFound => core_error::Code::NotFound,
            CannotIterate => core_error::Code::Internal,
            BackendError(_) => core_error::Code::Internal,
            Block0InFuture => core_error::Code::Internal,
        };
        Error {
            code,
            cause: err.into(),
        }
    }
}

impl From<Error> for core_error::Error {
    fn from(err: Error) -> Self {
        core_error::Error::new(err.code(), err)
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
    logger: Logger,
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
            Ok(Async::NotReady) => {
                return Ok(Async::NotReady);
            }
            Ok(Async::Ready(Ok(item))) => item,
            Ok(Async::Ready(Err(e))) => {
                warn!(self.logger, "error processing request: {:?}", e);
                return Err(Error::from(e).into());
            }
            Err(oneshot::Canceled) => {
                warn!(self.logger, "response canceled by the processing task");
                return Err(Error::from(oneshot::Canceled).into());
            }
        };

        Ok(Async::Ready(item))
    }
}

pub fn unary_reply<T, E>(logger: Logger) -> (ReplyHandle<T>, ReplyFuture<T, E>) {
    let (sender, receiver) = oneshot::channel();
    let future = ReplyFuture {
        receiver,
        logger,
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
    logger: Logger,
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
                warn!(self.logger, "error while streaming response: {:?}", e);
                return Err(e.into());
            }
        }
    }
}

pub fn stream_reply<T, E>(logger: Logger) -> (ReplyStreamHandle<T>, ReplyStream<T, E>) {
    let (sender, receiver) = mpsc::unbounded();
    let stream = ReplyStream {
        receiver,
        logger,
        _phantom_error: PhantomData,
    };
    (ReplyStreamHandle { sender }, stream)
}

pub fn do_stream_reply<T, F>(mut handler: ReplyStreamHandle<T>, f: F)
where
    F: FnOnce(&mut ReplyStreamHandle<T>) -> Result<(), Error>,
{
    match f(&mut handler) {
        Ok(()) => {}
        Err(e) => {
            handler.send_error(e);
        }
    };
    handler.close();
}

/// ...
#[derive(Debug)]
pub enum TransactionMsg {
    ProposeTransaction(Vec<FragmentId>, ReplyHandle<Vec<bool>>),
    SendTransaction(FragmentOrigin, Vec<Fragment>),
    GetTransactions(Vec<FragmentId>, ReplyStreamHandle<Fragment>),
}

/// Client messages, mainly requests from connected peers to our node.
/// Fetching the block headers, the block, the tip
pub enum ClientMsg {
    GetBlockTip(ReplyHandle<Header>),
    GetHeaders(Vec<HeaderHash>, ReplyStreamHandle<Header>),
    GetHeadersRange(Vec<HeaderHash>, HeaderHash, ReplyStreamHandle<Header>),
    GetBlocks(Vec<HeaderHash>, ReplyStreamHandle<Block>),
    GetBlocksRange(HeaderHash, HeaderHash, ReplyStreamHandle<Block>),
    PullBlocksToTip(Vec<HeaderHash>, ReplyStreamHandle<Block>),
}

impl Debug for ClientMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientMsg::GetBlockTip(_) => f
                .debug_tuple("GetBlockTip")
                .field(&format_args!("_"))
                .finish(),
            ClientMsg::GetHeaders(ids, _) => f
                .debug_tuple("GetHeaders")
                .field(ids)
                .field(&format_args!("_"))
                .finish(),
            ClientMsg::GetHeadersRange(from, to, _) => f
                .debug_tuple("GetHeadersRange")
                .field(from)
                .field(to)
                .field(&format_args!("_"))
                .finish(),
            ClientMsg::GetBlocks(ids, _) => f
                .debug_tuple("GetBlocksRange")
                .field(ids)
                .field(&format_args!("_"))
                .finish(),
            ClientMsg::GetBlocksRange(from, to, _) => f
                .debug_tuple("GetBlocksRange")
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
#[derive(Debug)]
pub enum BlockMsg {
    /// A trusted Block has been received from the leadership task
    LeadershipBlock(Block),
    /// Leadership process expect a new end of epoch
    LeadershipExpectEndOfEpoch(Epoch),
    /// A untrusted block Header has been received from the network task
    AnnouncedBlock(Header, NodeId),
    /// An untrusted Block has been received from the network task.
    /// The reply handle must be used to enable continued streaming by
    /// sending `Ok`, or to cancel the incoming stream with an error sent in
    /// `Err`.
    NetworkBlock(Block, ReplyHandle<()>),
    /// Headers for missing chain blocks have been received from the network
    /// in response to a PullHeaders request
    /// The reply handle must be used to enable continued streaming by
    /// sending `Ok`, or to cancel the incoming stream with an error sent in
    /// `Err`.
    ChainHeaders(Vec<Header>, ReplyHandle<()>),
}

/// Propagation requests for the network task.
#[derive(Clone, Debug)]
pub enum PropagateMsg {
    Block(Header),
    Message(Fragment),
}

/// Messages to the network task.
#[derive(Clone, Debug)]
pub enum NetworkMsg {
    Propagate(PropagateMsg),
    GetBlocks(Vec<HeaderHash>),
    GetNextBlock(NodeId, HeaderHash),
    PullHeaders {
        node_id: NodeId,
        from: Vec<HeaderHash>,
        to: HeaderHash,
    },
}

#[cfg(test)]
mod tests {}
