use crate::blockcfg::{Block, Epoch, Fragment, FragmentId, Header, HeaderHash};
use crate::network::p2p::comm::PeerStats;
use crate::network::p2p::topology::NodeId;
use crate::utils::async_msg::{self, MessageBox, MessageQueue};
use blockchain::Checkpoints;
use futures::prelude::*;
use futures::sync::{mpsc, oneshot};
use jormungandr_lib::interfaces::{FragmentOrigin, FragmentStatus};
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

    pub fn aborted<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: core_error::Code::Aborted,
            cause: cause.into(),
        }
    }

    pub fn canceled<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: core_error::Code::Canceled,
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

    pub fn invalid_argument<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: core_error::Code::InvalidArgument,
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
            BlockAlreadyPresent => core_error::Code::Internal,
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
        // Ignoring a send error: it means the result is no longer needed
        let _ = self.sender.send(result);
    }

    pub fn reply_ok(self, response: T) {
        self.reply(Ok(response))
    }

    pub fn reply_error(self, error: Error) {
        self.reply(Err(error))
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
        match self.receiver.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(Ok(item))) => {
                debug!(self.logger, "request processed");
                Ok(Async::Ready(item))
            }
            Ok(Async::Ready(Err(e))) => {
                info!(self.logger, "error processing request"; "reason" => %e);
                Err(e.into())
            }
            Err(oneshot::Canceled) => {
                warn!(self.logger, "response canceled by the processing task");
                Err(Error::from(oneshot::Canceled).into())
            }
        }
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
pub struct ReplySendError;

impl fmt::Display for ReplySendError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to send reply")
    }
}

impl error::Error for ReplySendError {}

#[derive(Debug)]
pub struct ReplyStreamHandle<T> {
    sender: mpsc::UnboundedSender<Result<T, Error>>,
}

impl<T> ReplyStreamHandle<T> {
    pub fn send(&mut self, item: T) -> Result<(), ReplySendError> {
        self.send_result(Ok(item))
    }

    pub fn send_error(&mut self, error: Error) -> Result<(), ReplySendError> {
        self.send_result(Err(error))
    }

    fn send_result(&mut self, res: Result<T, Error>) -> Result<(), ReplySendError> {
        self.sender.unbounded_send(res).map_err(|_| ReplySendError)
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
            if let Err(_) = handler.send_error(e) {
                return;
            }
        }
    };
    handler.close();
}

#[derive(Debug)]
pub struct RequestStreamHandle<T, R> {
    receiver: MessageQueue<T>,
    reply: ReplyHandle<R>,
}

pub struct RequestSink<T, R, E> {
    sender: MessageBox<T>,
    reply_future: Option<ReplyFuture<R, E>>,
    logger: Logger,
}

impl<T, R> RequestStreamHandle<T, R> {
    pub fn stream(&mut self) -> &mut MessageQueue<T> {
        &mut self.receiver
    }

    /// Drops the request stream and returns the reply handle.
    pub fn into_reply(self) -> ReplyHandle<R> {
        self.reply
    }
}

impl<T, R, E> RequestSink<T, R, E> {
    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    // This is for network which implements request_stream::MapResponse
    // for this type.
    pub fn take_reply_future(&mut self) -> ReplyFuture<R, E> {
        self.reply_future
            .take()
            .expect("there can be only one waiting for the reply")
    }
}

impl<T, R, E> RequestSink<T, R, E>
where
    E: From<Error>,
{
    fn map_send_error(&self, _e: mpsc::SendError<T>, msg: &'static str) -> E {
        debug!(self.logger, "{}", msg);
        Error::aborted("request stream processing ended before all items were sent").into()
    }
}

impl<T, R, E> Sink for RequestSink<T, R, E>
where
    E: From<Error>,
{
    type SinkItem = T;
    type SinkError = E;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.sender.start_send(item).map_err(|e| {
            self.map_send_error(
                e,
                "request stream processing ended before receiving some items",
            )
        })
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.sender.poll_complete().map_err(|e| {
            self.map_send_error(
                e,
                "request stream processing ended before receiving some items",
            )
        })
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.sender.close().map_err(|e| {
            self.map_send_error(
                e,
                "request stream processing channel did not close gracefully, \
                 the task possibly failed to receive some items",
            )
        })
    }
}

pub fn stream_request<T, R, E>(
    buffer: usize,
    logger: Logger,
) -> (RequestStreamHandle<T, R>, RequestSink<T, R, E>) {
    let (sender, receiver) = async_msg::channel(buffer);
    let (reply, reply_future) = unary_reply(logger.clone());
    let handle = RequestStreamHandle { receiver, reply };
    let sink = RequestSink {
        sender,
        reply_future: Some(reply_future),
        logger,
    };
    (handle, sink)
}

/// ...
#[derive(Debug)]
pub enum TransactionMsg {
    SendTransaction(FragmentOrigin, Vec<Fragment>),
    RemoveTransactions(Vec<FragmentId>, FragmentStatus),
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
    /// The stream of headers for missing chain blocks has been received
    /// from the network in response to a PullHeaders request or a Missing
    /// solicitation event.
    ChainHeaders(RequestStreamHandle<Header, ()>),
}

/// Propagation requests for the network task.
#[derive(Clone, Debug)]
pub enum PropagateMsg {
    Block(Header),
    Fragment(Fragment),
}

/// Messages to the network task.
#[derive(Debug)]
pub enum NetworkMsg {
    Propagate(PropagateMsg),
    GetBlocks(Vec<HeaderHash>),
    GetNextBlock(NodeId, HeaderHash),
    PullHeaders {
        node_id: NodeId,
        from: Checkpoints,
        to: HeaderHash,
    },
    PeerStats(ReplyHandle<Vec<(NodeId, PeerStats)>>),
}

/// Messages to the explorer task
pub enum ExplorerMsg {
    NewBlock(Block),
}

#[cfg(test)]
mod tests {}
