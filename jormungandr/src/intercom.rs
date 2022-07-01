use crate::{
    blockcfg::{ApplyBlockLedger, Block, Fragment, FragmentId, Header, HeaderHash},
    blockchain::{Checkpoints, LeadershipBlock, StorageError},
    fragment::selection::FragmentSelectionAlgorithmParams,
    network::p2p::comm::PeerInfo,
    topology::{Gossips, NodeId, Peer, PeerInfo as TopologyPeerInfo, View},
    utils::async_msg::{self, MessageBox, MessageQueue},
};
use chain_impl_mockchain::fragment::Contents as FragmentContents;
use chain_network::error as net_error;
use futures::{
    channel::{mpsc, oneshot},
    prelude::*,
    ready,
};
use jormungandr_lib::interfaces::{
    BlockDate, FragmentLog, FragmentOrigin, FragmentStatus, FragmentsProcessingSummary,
};
use poldercast::layer::Selection;
use std::{
    collections::HashMap,
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

impl From<StorageError> for Error {
    fn from(err: StorageError) -> Self {
        let code = match &err {
            StorageError::BlockNotFound => net_error::Code::NotFound,
            StorageError::CannotIterate => net_error::Code::Internal,
            StorageError::BackendError(_) => net_error::Code::Internal,
            StorageError::BlockAlreadyPresent => net_error::Code::Internal,
            StorageError::MissingParent => net_error::Code::InvalidArgument,
            StorageError::Deserialize(_) => net_error::Code::Internal,
            StorageError::Serialize(_) => net_error::Code::Internal,
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
}

impl<T> Unpin for ReplyFuture<T> {}

impl<T> Future for ReplyFuture<T> {
    type Output = Result<T, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<T, Error>> {
        Pin::new(&mut self.receiver).poll(cx).map(|res| match res {
            Ok(Ok(item)) => {
                tracing::debug!("request processed");
                Ok(item)
            }
            Ok(Err(e)) => {
                tracing::info!(reason = %e, "error processing request");
                Err(e)
            }
            Err(oneshot::Canceled) => {
                tracing::warn!("response canceled by the processing task");
                Err(Error::from(oneshot::Canceled))
            }
        })
    }
}

pub fn unary_reply<T>() -> (ReplyHandle<T>, ReplyFuture<T>) {
    let (sender, receiver) = oneshot::channel();
    let future = ReplyFuture { receiver };
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
    lead_sender: oneshot::Sender<Result<mpsc::Receiver<Result<T, Error>>, Error>>,
    buffer_size: usize,
}

impl<T> ReplyStreamHandle<T> {
    fn reply(self, result: Result<mpsc::Receiver<Result<T, Error>>, Error>) {
        // Ignoring a send error: it means the result is no longer needed
        let _ = self.lead_sender.send(result);
    }

    pub fn start_sending(self) -> ReplyStreamSink<T> {
        let (sender, receiver) = mpsc::channel(self.buffer_size);
        self.reply(Ok(receiver));
        ReplyStreamSink { sender }
    }

    pub fn reply_error(self, error: Error) {
        self.reply(Err(error))
    }
}

#[derive(Debug)]
pub struct ReplyStreamSink<T> {
    sender: mpsc::Sender<Result<T, Error>>,
}

impl<T> Unpin for ReplyStreamSink<T> {}

impl<T> Clone for ReplyStreamSink<T> {
    fn clone(&self) -> Self {
        ReplyStreamSink {
            sender: self.sender.clone(),
        }
    }
}

impl<T> ReplyStreamSink<T> {
    pub fn try_send_item(&mut self, item: Result<T, Error>) -> Result<(), ReplyTrySendError<T>> {
        self.sender.try_send(item).map_err(ReplyTrySendError)
    }

    pub fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), ReplySendError>> {
        self.sender.poll_ready(cx).map_err(|_| ReplySendError)
    }
}

impl<T> Sink<Result<T, Error>> for ReplyStreamSink<T> {
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

pub struct ReplyStreamFuture<T, E> {
    lead_receiver: oneshot::Receiver<Result<mpsc::Receiver<Result<T, Error>>, Error>>,
    _phantom_error: PhantomData<E>,
}

impl<T, E> Unpin for ReplyStreamFuture<T, E> {}

impl<T, E> Future for ReplyStreamFuture<T, E>
where
    E: From<Error>,
{
    type Output = Result<ReplyStream<T, E>, E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let receiver = ready!(Pin::new(&mut self.lead_receiver).poll(cx)).map_err(
            |e: oneshot::Canceled| {
                tracing::warn!("response canceled by the processing task");
                Error::from(e)
            },
        )??;
        let stream = ReplyStream {
            receiver,
            _phantom_error: PhantomData,
        };
        Poll::Ready(Ok(stream))
    }
}

pub struct ReplyStream<T, E> {
    receiver: mpsc::Receiver<Result<T, Error>>,
    _phantom_error: PhantomData<E>,
}

impl<T, E> Unpin for ReplyStream<T, E> {}

impl<T> ReplyStream<T, Error> {
    /// Converts this stream into an infallible stream for uploading
    pub fn upload(self) -> UploadStream<T> {
        UploadStream { inner: self }
    }
}

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
                    tracing::info!(
                        error = ?e,
                        "error while streaming response"
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

pub fn stream_reply<T, E>(buffer_size: usize) -> (ReplyStreamHandle<T>, ReplyStreamFuture<T, E>) {
    let (lead_sender, lead_receiver) = oneshot::channel();
    let handle = ReplyStreamHandle {
        lead_sender,
        buffer_size,
    };
    let future = ReplyStreamFuture {
        lead_receiver,
        _phantom_error: PhantomData,
    };
    (handle, future)
}

#[derive(Debug)]
pub struct RequestStreamHandle<T, R> {
    receiver: MessageQueue<T>,
    reply: ReplyHandle<R>,
}

pub struct RequestSink<T> {
    sender: MessageBox<T>,
}

impl<T, R> RequestStreamHandle<T, R> {
    pub fn into_stream_and_reply(self) -> (MessageQueue<T>, ReplyHandle<R>) {
        (self.receiver, self.reply)
    }
}

impl<T> RequestSink<T> {
    fn map_send_error(&self, _e: mpsc::SendError, msg: &'static str) -> Error {
        tracing::debug!("{}", msg);
        Error::aborted("request stream processing ended before all items were sent")
    }
}

impl<T> Sink<T> for RequestSink<T> {
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
) -> (RequestStreamHandle<T, R>, RequestSink<T>, ReplyFuture<R>) {
    let (sender, receiver) = async_msg::channel(buffer);
    let (reply, reply_future) = unary_reply();
    let handle = RequestStreamHandle { receiver, reply };
    let sink = RequestSink { sender };
    (handle, sink, reply_future)
}

/// ...
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum TransactionMsg {
    SendTransactions {
        origin: FragmentOrigin,
        fragments: Vec<Fragment>,
        fail_fast: bool,
        reply_handle: ReplyHandle<FragmentsProcessingSummary>,
    },
    RemoveTransactions(Vec<FragmentId>, FragmentStatus),
    BranchSwitch(BlockDate),
    GetLogs(ReplyHandle<Vec<FragmentLog>>),
    GetStatuses(
        Vec<FragmentId>,
        ReplyHandle<HashMap<FragmentId, FragmentStatus>>,
    ),
    SelectTransactions {
        ledger: ApplyBlockLedger,
        selection_alg: FragmentSelectionAlgorithmParams,
        reply_handle: ReplyHandle<(FragmentContents, ApplyBlockLedger)>,
        soft_deadline_future: futures::channel::oneshot::Receiver<()>,
        hard_deadline_future: futures::channel::oneshot::Receiver<()>,
    },
}

/// Client messages, mainly requests from connected peers to our node.
/// Fetching the block headers, the block, the tip
pub enum ClientMsg {
    GetBlockTip(ReplyHandle<Header>),
    GetHeaders(Vec<HeaderHash>, ReplyStreamHandle<Header>),
    PullHeaders(Vec<HeaderHash>, HeaderHash, ReplyStreamHandle<Header>),
    GetBlocks(Vec<HeaderHash>, ReplyStreamHandle<Block>),
    PullBlocks(Vec<HeaderHash>, HeaderHash, ReplyStreamHandle<Block>),
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
            ClientMsg::PullHeaders(from, to, _) => f
                .debug_tuple("PullHeaders")
                .field(from)
                .field(to)
                .field(&format_args!("_"))
                .finish(),
            ClientMsg::GetBlocks(ids, _) => f
                .debug_tuple("GetBlocks")
                .field(ids)
                .field(&format_args!("_"))
                .finish(),
            ClientMsg::PullBlocks(from, to, _) => f
                .debug_tuple("PullBlocks")
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
pub enum BlockMsg {
    /// A trusted Block has been received from the leadership task
    LeadershipBlock(Box<LeadershipBlock>),
    /// A untrusted block Header has been received from the network task
    AnnouncedBlock(Box<Header>, NodeId),
    /// A stream of untrusted blocks has been received from the network task.
    NetworkBlocks(RequestStreamHandle<Block, ()>),
    /// The stream of headers for missing chain blocks has been received
    /// from the network in response to a PullHeaders request or a Missing
    /// solicitation event.
    ChainHeaders(RequestStreamHandle<Header, ()>),
}

/// Propagation requests for the network task.
#[derive(Debug)]
pub enum PropagateMsg {
    Block(Box<Header>),
    Fragment(Fragment),
    Gossip(Peer, Gossips),
}

/// Messages to the network task.
#[derive(Debug)]
pub enum NetworkMsg {
    Propagate(Box<PropagateMsg>),
    GetBlocks(Vec<HeaderHash>),
    GetNextBlock(NodeId, HeaderHash),
    PullHeaders {
        node_id: NodeId,
        from: Checkpoints,
        to: HeaderHash,
    },
    PeerInfo(ReplyHandle<Vec<PeerInfo>>),
}

/// Messages to the topology task
pub enum TopologyMsg {
    AcceptGossip(Gossips),
    DemotePeer(NodeId),
    PromotePeer(NodeId),
    View(Selection, ReplyHandle<View>),
    ListAvailable(ReplyHandle<Vec<TopologyPeerInfo>>),
    ListNonPublic(ReplyHandle<Vec<TopologyPeerInfo>>),
    ListQuarantined(ReplyHandle<Vec<TopologyPeerInfo>>),
}

/// Messages to the notifier task
pub enum WatchMsg {
    NewBlock(Block),
    NewTip(Header),
}

#[cfg(test)]
mod tests {}
