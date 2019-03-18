use crate::blockcfg::BlockConfig;

use network_core::{error::Code, server::block::BlockError};

use futures::prelude::*;
use futures::sync::{mpsc, oneshot};
use tokio_bus::BusReader;

use std::{
    error,
    fmt::{self, Debug, Display},
    marker::PhantomData,
    sync::mpsc::RecvError as BusError,
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

pub struct SubscriptionHandle<T: Sync + Clone> {
    sender: oneshot::Sender<BusReader<T>>,
}

impl<T: Sync + Clone> SubscriptionHandle<T> {
    pub fn send(self, rx: BusReader<T>) {
        match self.sender.send(rx) {
            Ok(()) => {}
            Err(_) => panic!("failed to send subscription reader"),
        }
    }
}

pub struct SubscriptionFuture<T, E>
where
    T: Clone + Sync,
{
    receiver: oneshot::Receiver<BusReader<T>>,
    _phantom_error: PhantomData<E>,
}

impl<T, E> Future for SubscriptionFuture<T, E>
where
    T: Clone + Sync,
    E: From<Error>,
{
    type Item = SubscriptionStream<T, E>;
    type Error = E;
    fn poll(&mut self) -> Poll<Self::Item, E> {
        let inner = match self.receiver.poll() {
            Err(oneshot::Canceled) => {
                warn!("response canceled by the client request task");
                return Err(Error::from(oneshot::Canceled).into());
            }
            Ok(Async::NotReady) => {
                return Ok(Async::NotReady);
            }
            Ok(Async::Ready(item)) => item,
        };

        Ok(Async::Ready(SubscriptionStream {
            inner,
            _phantom_error: PhantomData,
        }))
    }
}

pub struct SubscriptionStream<T: Clone + Sync, E> {
    inner: BusReader<T>,
    _phantom_error: PhantomData<E>,
}

impl<T, E> Stream for SubscriptionStream<T, E>
where
    T: Clone + Sync,
    E: FromSubscriptionError,
{
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.inner.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(item)) => Ok(Async::Ready(item)),
            Err(e) => Err(E::from_subscription_error(e)),
        }
    }
}

pub trait FromSubscriptionError: Sized {
    fn from_subscription_error(err: BusError) -> Self;
}

impl FromSubscriptionError for BlockError {
    fn from_subscription_error(err: BusError) -> Self {
        BlockError::failed(err)
    }
}

pub fn subscription_reply<T, E>() -> (SubscriptionHandle<T>, SubscriptionFuture<T, E>)
where
    T: Clone + Sync,
{
    let (sender, receiver) = oneshot::channel();
    let future = SubscriptionFuture {
        receiver,
        _phantom_error: PhantomData,
    };
    (SubscriptionHandle { sender }, future)
}

/// ...
#[derive(Debug)]
pub enum TransactionMsg<B: BlockConfig> {
    ProposeTransaction(Vec<B::MessageId>, ReplyHandle<Vec<bool>>),
    SendTransaction(Vec<B::Message>),
    GetTransactions(Vec<B::MessageId>, ReplyStreamHandle<B::Message>),
}

/// Client messages, mainly requests from connected peers to our node.
/// Fetching the block headers, the block, the tip
pub enum ClientMsg<B: BlockConfig> {
    GetBlockTip(ReplyHandle<B::BlockHeader>),
    GetHeaders(Vec<B::BlockHash>, ReplyStreamHandle<B::BlockHeader>),
    GetHeadersRange(
        Vec<B::BlockHash>,
        B::BlockHash,
        ReplyHandle<Vec<B::BlockHeader>>,
    ),
    GetBlocks(Vec<B::BlockHash>, ReplyStreamHandle<B::Block>),
    GetBlocksRange(B::BlockHash, B::BlockHash, ReplyStreamHandle<B::Block>),
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
pub enum BlockMsg<B: BlockConfig> {
    /// A trusted Block has been received from the leadership task
    LeadershipBlock(B::Block),
    /// The network task has a subscription to add
    Subscribe(SubscriptionHandle<B::BlockHeader>),
    /// A untrusted block Header has been received from the network task
    AnnouncedBlock(B::BlockHeader),
}

impl<B> Debug for BlockMsg<B>
where
    B: BlockConfig,
    B::Block: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use BlockMsg::*;
        match self {
            LeadershipBlock(block) => f.debug_tuple("LeadershipBlock").field(block).finish(),
            Subscribe(_) => f.debug_tuple("Subscribe").finish(),
            AnnouncedBlock(header) => f.debug_tuple("AnnouncedBlock").field(header).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockcfg::mock::Mockchain;
    use chain_impl_mockchain::block::Header;
    use network_core::server::block::BlockError;

    #[test]
    fn block_msg_subscribe_debug() {
        let (handle, _) = subscription_reply::<Header, BlockError>();
        let msg = BlockMsg::Subscribe::<Mockchain>(handle);
        let debug_repr = format!("{:?}", msg);
        assert!(debug_repr.contains("Subscribe"));
    }
}
