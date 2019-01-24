use crate::gen::{self, node::client as gen_client};

use chain_core::property::{Block, BlockDate, BlockId, Deserialize, Serialize};
use network_core::client::{self as core_client, block::BlockService};

use futures::future::Executor;
use tokio::io;
use tokio::prelude::*;
use tower_grpc::{BoxBody, Request, Response, Streaming};
use tower_h2::client::{Background, Connect, ConnectError, Connection};
use tower_util::MakeService;

use std::{error, fmt, marker::PhantomData, mem, str::FromStr};

/// gRPC client for blockchain node.
///
/// This type encapsulates the gRPC protocol client that can
/// make connections and perform requests towards other blockchain nodes.
pub struct Client<S, E> {
    node: gen_client::Node<Connection<S, E, BoxBody>>,
}

impl<S, E> Client<S, E>
where
    S: AsyncRead + AsyncWrite,
    E: Executor<Background<S, BoxBody>> + Clone,
{
    pub fn connect<P>(peer: P, executor: E) -> impl Future<Item = Self, Error = Error>
    where
        P: tokio_connect::Connect<Connected = S, Error = io::Error> + 'static,
    {
        let mut make_client = Connect::new(peer, Default::default(), executor);
        make_client
            .make_service(())
            .map_err(|e| Error::Connect(e))
            .map(|conn| {
                // TODO: add origin URL with add_origin middleware from tower-http

                Client {
                    node: gen_client::Node::new(conn),
                }
            })
    }
}

type GrpcFuture<R> = tower_grpc::client::unary::ResponseFuture<
    R,
    tower_h2::client::ResponseFuture,
    tower_h2::RecvBody,
>;

type GrpcStreamFuture<R> =
    tower_grpc::client::server_streaming::ResponseFuture<R, tower_h2::client::ResponseFuture>;

type GrpcError = tower_grpc::Error<tower_h2::client::Error>;

type GrpcStreamError = tower_grpc::Error<()>;

pub enum ResponseFuture<T, R> {
    Pending(GrpcFuture<R>),
    Finished(PhantomData<T>),
}

impl<T, R> ResponseFuture<T, R> {
    fn new(future: GrpcFuture<R>) -> Self {
        ResponseFuture::Pending(future)
    }
}

pub enum ResponseStreamFuture<T, R> {
    Pending(GrpcStreamFuture<R>),
    Finished(PhantomData<T>),
}

impl<T, R> ResponseStreamFuture<T, R> {
    fn new(future: GrpcStreamFuture<R>) -> Self {
        ResponseStreamFuture::Pending(future)
    }
}

pub struct ResponseStream<T, R> {
    inner: Streaming<R, tower_h2::RecvBody>,
    _phantom: PhantomData<T>,
}

fn convert_error(_e: GrpcError) -> core_client::Error {
    core_client::Error::Rpc
}

fn convert_stream_error(_e: GrpcStreamError) -> core_client::Error {
    core_client::Error::Rpc
}

pub trait ConvertResponse<T> {
    fn convert_item(self) -> Result<T, core_client::Error>;
}

fn poll_and_convert_response<T, R, F>(future: &mut F) -> Poll<T, core_client::Error>
where
    F: Future<Item = Response<R>, Error = GrpcError>,
    R: ConvertResponse<T>,
{
    match future.poll() {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(res)) => {
            let item = res.into_inner().convert_item()?;
            Ok(Async::Ready(item))
        }
        Err(e) => Err(convert_error(e)),
    }
}

fn poll_and_convert_stream_future<T, R, F>(
    future: &mut F,
) -> Poll<ResponseStream<T, R>, core_client::Error>
where
    F: Future<Item = Response<Streaming<R, tower_h2::RecvBody>>, Error = GrpcError>,
{
    match future.poll() {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(res)) => {
            let stream = ResponseStream {
                inner: res.into_inner(),
                _phantom: PhantomData,
            };
            Ok(Async::Ready(stream))
        }
        Err(e) => Err(convert_error(e)),
    }
}

fn poll_and_convert_stream<T, S, R>(stream: &mut S) -> Poll<Option<T>, core_client::Error>
where
    S: Stream<Item = R, Error = GrpcStreamError>,
    R: ConvertResponse<T>,
{
    match stream.poll() {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
        Ok(Async::Ready(Some(item))) => {
            let item = item.convert_item()?;
            Ok(Async::Ready(Some(item)))
        }
        Err(e) => Err(convert_stream_error(e)),
    }
}

impl<T, R> Future for ResponseFuture<T, R>
where
    R: prost::Message + Default + ConvertResponse<T>,
{
    type Item = T;
    type Error = core_client::Error;

    fn poll(&mut self) -> Poll<T, core_client::Error> {
        if let ResponseFuture::Pending(f) = self {
            let res = poll_and_convert_response(f);
            if let Ok(Async::NotReady) = res {
                return Ok(Async::NotReady);
            }
            *self = ResponseFuture::Finished(PhantomData);
            res
        } else {
            match mem::replace(self, ResponseFuture::Finished(PhantomData)) {
                ResponseFuture::Pending(_) => unreachable!(),
                ResponseFuture::Finished(_) => panic!("polled a finished response"),
            }
        }
    }
}

impl<T, R> Future for ResponseStreamFuture<T, R>
where
    R: prost::Message + Default,
{
    type Item = ResponseStream<T, R>;
    type Error = core_client::Error;

    fn poll(&mut self) -> Poll<ResponseStream<T, R>, core_client::Error> {
        if let ResponseStreamFuture::Pending(f) = self {
            let res = poll_and_convert_stream_future(f);
            if let Ok(Async::NotReady) = res {
                return Ok(Async::NotReady);
            }
            *self = ResponseStreamFuture::Finished(PhantomData);
            res
        } else {
            match mem::replace(self, ResponseStreamFuture::Finished(PhantomData)) {
                ResponseStreamFuture::Pending(_) => unreachable!(),
                ResponseStreamFuture::Finished(_) => panic!("polled a finished response"),
            }
        }
    }
}

impl<T, R> Stream for ResponseStream<T, R>
where
    R: prost::Message + Default + ConvertResponse<T>,
{
    type Item = T;
    type Error = core_client::Error;

    fn poll(&mut self) -> Poll<Option<T>, core_client::Error> {
        poll_and_convert_stream(&mut self.inner)
    }
}

fn deserialize_bytes<T>(mut buf: &[u8]) -> Result<T, core_client::Error>
where
    T: Deserialize,
{
    T::deserialize(&mut buf).map_err(|_e| core_client::Error::Format)
}

fn parse_str<T>(s: &str) -> Result<T, core_client::Error>
where
    T: FromStr,
{
    T::from_str(s).map_err(|_e| core_client::Error::Format)
}

fn serialize_to_vec<T>(values: &[T]) -> Vec<Vec<u8>>
where
    T: Serialize,
{
    values
        .iter()
        .map(|x| {
            let mut v = Vec::new();
            x.serialize(&mut v).unwrap();
            v
        })
        .collect()
}

impl<I, D> ConvertResponse<(I, D)> for gen::node::TipResponse
where
    I: BlockId + Deserialize,
    D: BlockDate + FromStr,
{
    fn convert_item(self) -> Result<(I, D), core_client::Error> {
        let id = deserialize_bytes(&self.id)?;
        let blockdate = parse_str(&self.blockdate)?;
        Ok((id, blockdate))
    }
}

impl<T> ConvertResponse<T> for gen::node::Block
where
    T: Block,
{
    fn convert_item(self) -> Result<T, core_client::Error> {
        let block = deserialize_bytes(&self.content)?;
        Ok(block)
    }
}

impl<T, S, E> BlockService<T> for Client<S, E>
where
    T: Block,
    S: AsyncRead + AsyncWrite,
    E: Executor<Background<S, BoxBody>> + Clone,
    <T as Block>::Date: FromStr,
{
    type TipFuture = ResponseFuture<(T::Id, T::Date), gen::node::TipResponse>;

    type StreamBlocksToTipStream = ResponseStream<T, gen::node::Block>;
    type StreamBlocksToTipFuture = ResponseStreamFuture<T, gen::node::Block>;

    fn tip(&mut self) -> Self::TipFuture {
        let req = gen::node::TipRequest {};
        let future = self.node.tip(Request::new(req));
        ResponseFuture::new(future)
    }

    fn stream_blocks_to_tip(&mut self, from: &[T::Id]) -> Self::StreamBlocksToTipFuture {
        let from = serialize_to_vec(from);
        let req = gen::node::StreamBlocksToTipRequest { from };
        let future = self.node.stream_blocks_to_tip(Request::new(req));
        ResponseStreamFuture::new(future)
    }
}

/// The error type for gRPC client operations.
#[derive(Debug)]
pub enum Error {
    Connect(ConnectError<io::Error>),
}

impl From<ConnectError<io::Error>> for Error {
    fn from(err: ConnectError<io::Error>) -> Self {
        Error::Connect(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Connect(e) => write!(f, "connection error: {}", e),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Connect(e) => Some(e),
        }
    }
}
