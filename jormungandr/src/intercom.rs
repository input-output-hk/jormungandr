use crate::blockcfg::{
    Block, BlockDate, Fragment, FragmentId, Header, HeaderHash, Ledger, LedgerParameters,
};
use crate::blockchain::Checkpoints;
use crate::fragment::selection::FragmentSelectionAlgorithmParams;
use crate::network::p2p::{comm::PeerInfo, Address};
use crate::utils::async_msg::{self, MessageBox, MessageQueue};
use chain_impl_mockchain::fragment::Contents as FragmentContents;
use chain_network::error as net_error;
use jormungandr_lib::interfaces::{FragmentLog, FragmentOrigin, FragmentStatus};

use futures::channel::{mpsc, oneshot};
use futures::prelude::*;
use slog::Logger;
use std::{
    error,
    fmt::{self, Debug, Display},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

/// The error values passed via intercom messages.
#[derive(Debug)]
pub struct Error {
    code: net_error::Code,
    cause: Box<dyn error::Error + Send + Sync>,
}

impl Error {
    pub fn failed<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: net_error::Code::Internal,
            cause: cause.into(),
        }
    }

    pub fn aborted<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: net_error::Code::Aborted,
            cause: cause.into(),
        }
    }

    pub fn canceled<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: net_error::Code::Canceled,
            cause: cause.into(),
        }
    }

    pub fn failed_precondition<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: net_error::Code::FailedPrecondition,
            cause: cause.into(),
        }
    }

    pub fn invalid_argument<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: net_error::Code::InvalidArgument,
            cause: cause.into(),
        }
    }

    pub fn not_found<T>(cause: T) -> Self
    where
        T: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            code: net_error::Code::NotFound,
            cause: cause.into(),
        }
    }

    pub fn unimplemented<S: Into<String>>(message: S) -> Self {
        Error {
            code: net_error::Code::Unimplemented,
            cause: message.into().into(),
        }
    }

    pub fn code(&self) -> net_error::Code {
        self.code
    }
}

impl From<oneshot::Canceled> for Error {
    fn from(src: oneshot::Canceled) -> Self {
        Error {
            code: net_error::Code::Unavailable,
            cause: src.into(),
        }
    }
}

impl From<chain_storage::Error> for Error {
    fn from(err: chain_storage::Error) -> Self {
        use chain_storage::Error::*;

        let code = match err {
            BlockNotFound => net_error::Code::NotFound,
            CannotIterate => net_error::Code::Internal,
            BackendError(_) => net_error::Code::Internal,
            Block0InFuture => net_error::Code::Internal,
            BlockAlreadyPresent => net_error::Code::Internal,
            MissingParent => net_error::Code::InvalidArgument,
        };
        Error {
            code,
            cause: err.into(),
        }
    }
}

impl From<Error> for net_error::Error {
    fn from(err: Error) -> Self {
        net_error::Error::new(err.code(), err.cause)
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

pub struct ReplyFuture<T> {
    receiver: oneshot::Receiver<Result<T, Error>>,
    logger: Logger,
}

impl<T> Unpin for ReplyFuture<T> {}

impl<T> Future for ReplyFuture<T> {
    type Output = Result<T, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<T, Error>> {
        Pin::new(&mut self.receiver).poll(cx).map(|res| match res {
            Ok(Ok(item)) => {
                debug!(self.logger, "request processed");
                Ok(item)
            }
            Ok(Err(e)) => {
                info!(self.logger, "error processing request"; "reason" => %e);
                Err(e)
            }
            Err(oneshot::Canceled) => {
                warn!(self.logger, "response canceled by the processing task");
                Err(Error::from(oneshot::Canceled))
            }
        })
    }
}

pub fn unary_reply<T>(logger: Logger) -> (ReplyHandle<T>, ReplyFuture<T>) {
    let (sender, receiver) = oneshot::channel();
    let future = ReplyFuture { receiver, logger };
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

pub struct ReplyTrySendError<T>(mpsc::TrySendError<Result<T, Error>>);

impl<T> ReplyTrySendError<T> {
    pub fn is_full(&self) -> bool {
        self.0.is_full()
    }

    pub fn into_inner(self) -> Result<T, Error> {
        self.0.into_inner()
    }

    pub fn into_send_error(self) -> ReplySendError {
        ReplySendError
    }
}

impl<T> fmt::Debug for ReplyTrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("ReplyTrySendError").field(&self.0).finish()
    }
}

impl<T> fmt::Display for ReplyTrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to send reply")
    }
}

impl<T: 'static> error::Error for ReplyTrySendError<T> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.0)
    }
}

#[derive(Debug)]
pub struct ReplyStreamHandle<T> {
    sender: mpsc::Sender<Result<T, Error>>,
}

impl<T> Unpin for ReplyStreamHandle<T> {}

impl<T> Clone for ReplyStreamHandle<T> {
    fn clone(&self) -> Self {
        ReplyStreamHandle {
            sender: self.sender.clone(),
        }
    }
}

impl<T> ReplyStreamHandle<T> {
    pub fn try_send_item(&mut self, item: Result<T, Error>) -> Result<(), ReplyTrySendError<T>> {
        self.sender.try_send(item).map_err(ReplyTrySendError)
    }

    pub fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), ReplySendError>> {
        self.sender.poll_ready(cx).map_err(|_| ReplySendError)
    }
}

impl<T> Sink<Result<T, Error>> for ReplyStreamHandle<T> {
    type Error = ReplySendError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.sender)
            .poll_ready(cx)
            .map_err(|_| ReplySendError)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Result<T, Error>) -> Result<(), Self::Error> {
        Pin::new(&mut self.sender)
            .start_send(item)
            .map_err(|_| ReplySendError)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.sender)
            .poll_flush(cx)
            .map_err(|_| ReplySendError)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.sender)
            .poll_close(cx)
            .map_err(|_| ReplySendError)
    }
}

pub struct ReplyStream<T, E> {
    receiver: mpsc::Receiver<Result<T, Error>>,
    logger: Logger,
    _phantom_error: PhantomData<E>,
}

impl<T, E> Unpin for ReplyStream<T, E> {}

impl<T, E> Stream for ReplyStream<T, E>
where
    E: From<Error>,
{
    type Item = Result<T, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver)
            .poll_next(cx)
            .map(|maybe_res| match maybe_res {
                Some(Ok(item)) => Some(Ok(item)),
                None => None,
                Some(Err(e)) => {
                    info!(
                        self.logger,
                        "error while streaming response";
                        "error" => ?e,
                    );
                    Some(Err(e.into()))
                }
            })
    }
}

/// An adapter for outbound client streaming requests
pub struct UploadStream<T> {
    inner: ReplyStream<T, Error>,
}

impl<T> Stream for UploadStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<T>> {
        Pin::new(&mut self.inner)
            .poll_next(cx)
            .map(|maybe_res| match maybe_res {
                Some(Ok(item)) => Some(item),
                None => None,
                Some(Err(_)) => None,
            })
    }
}

pub fn stream_reply<T, E>(
    buffer: usize,
    logger: Logger,
) -> (ReplyStreamHandle<T>, ReplyStream<T, E>) {
    let (sender, receiver) = mpsc::channel(buffer);
    let stream = ReplyStream {
        receiver,
        logger,
        _phantom_error: PhantomData,
    };
    (ReplyStreamHandle { sender }, stream)
}

pub fn upload_stream_reply<T>(
    buffer: usize,
    logger: Logger,
) -> (ReplyStreamHandle<T>, UploadStream<T>) {
    let (handle, inner) = stream_reply(buffer, logger);
    (handle, UploadStream { inner })
}

#[derive(Debug)]
pub struct RequestStreamHandle<T, R> {
    receiver: MessageQueue<T>,
    reply: ReplyHandle<R>,
}

pub struct RequestSink<T, R> {
    sender: MessageBox<T>,
    reply_future: Option<ReplyFuture<R>>,
    logger: Logger,
}

impl<T, R> RequestStreamHandle<T, R> {
    pub fn into_stream_and_reply(self) -> (MessageQueue<T>, ReplyHandle<R>) {
        (self.receiver, self.reply)
    }
}

impl<T, R> RequestSink<T, R> {
    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    // This is for network which implements request_stream::MapResponse
    // for this type.
    pub fn take_reply_future(&mut self) -> ReplyFuture<R> {
        self.reply_future
            .take()
            .expect("there can be only one waiting for the reply")
    }
}

impl<T, R> RequestSink<T, R> {
    fn map_send_error(&self, _e: mpsc::SendError, msg: &'static str) -> Error {
        debug!(self.logger, "{}", msg);
        Error::aborted("request stream processing ended before all items were sent").into()
    }
}

impl<T, R> Sink<T> for RequestSink<T, R> {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        self.sender.poll_ready(cx).map_err(|e| {
            self.map_send_error(
                e,
                "request stream processing ended before receiving some items",
            )
        })
    }

    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Error> {
        self.sender.start_send(item).map_err(|e| {
            self.map_send_error(
                e,
                "request stream processing ended before receiving some items",
            )
        })
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.sender).poll_flush(cx).map_err(|e| {
            self.map_send_error(
                e,
                "request stream processing ended before receiving some items",
            )
        })
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.sender).poll_close(cx).map_err(|e| {
            self.map_send_error(
                e,
                "request stream processing channel did not close gracefully, \
                 the task possibly failed to receive some items",
            )
        })
    }
}

pub fn stream_request<T, R>(
    buffer: usize,
    logger: Logger,
) -> (RequestStreamHandle<T, R>, RequestSink<T, R>) {
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
    GetLogs(ReplyHandle<Vec<FragmentLog>>),
    SelectTransactions {
        ledger: Ledger,
        block_date: BlockDate,
        ledger_params: LedgerParameters,
        selection_alg: FragmentSelectionAlgorithmParams,
        reply_handle: ReplyHandle<FragmentContents>,
    },
}

/// Client messages, mainly requests from connected peers to our node.
/// Fetching the block headers, the block, the tip
pub enum ClientMsg {
    GetBlockTip(ReplyHandle<Header>),
    GetHeaders(Vec<HeaderHash>, ReplyStreamHandle<Header>),
    GetHeadersRange(Vec<HeaderHash>, HeaderHash, ReplyStreamHandle<Header>),
    GetBlocks(Vec<HeaderHash>, ReplyStreamHandle<Block>),
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
                .debug_tuple("GetBlocks")
                .field(ids)
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
    /// A untrusted block Header has been received from the network task
    AnnouncedBlock(Header, Address),
    /// A stream of untrusted blocks has been received from the network task.
    NetworkBlocks(RequestStreamHandle<Block, ()>),
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
    GetNextBlock(Address, HeaderHash),
    PullHeaders {
        node_address: Address,
        from: Checkpoints,
        to: HeaderHash,
    },
    PeerInfo(ReplyHandle<Vec<PeerInfo>>),
}

/// Messages to the explorer task
pub enum ExplorerMsg {
    NewBlock(Block),
}

#[cfg(test)]
mod tests {}
