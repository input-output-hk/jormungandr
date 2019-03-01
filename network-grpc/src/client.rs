use crate::gen::{self, node::client as gen_client};

use chain_core::property::{self, Deserialize, FromStr, HasHeader, Serialize};
use network_core::client::{self as core_client, block::BlockService};

use futures::future::Executor;
use tokio::io;
use tokio::prelude::*;
use tower_grpc::{BoxBody, Request, Streaming};
use tower_h2::client::{Background, Connect, ConnectError, Connection};
use tower_service::Service;
use tower_util::MakeService;

use std::{
    error,
    fmt::{self, Debug},
    marker::PhantomData,
};

/// Traits setting additional bounds for blockchain entities
/// that need to be satisfied for the protocol implementation.
///
/// The traits are auto-implemented for the types that satisfy the necessary
/// bounds. These traits then can be used in lieu of the lengthy bound clauses,
/// so that, should the implementation requrements change, only these trait
/// definitions and blanket implementations need to be modified.
pub mod chain_bounds {
    use chain_core::property;

    pub trait BlockId: property::BlockId
    // Alas, bounds on associated types of the supertrait do not have
    // the desired effect:
    // https://github.com/rust-lang/rust/issues/32722
    //
    // where
    //    <Self as property::Deserialize>::Error: Send + Sync,
    {
    }

    impl<T> BlockId for T where T: property::BlockId + property::Deserialize {}

    pub trait BlockDate: property::BlockDate + property::FromStr {}

    impl<T> BlockDate for T where T: property::BlockDate + property::FromStr {}

    pub trait Header: property::Header + property::Deserialize {}

    impl<T> Header for T
    where
        T: property::Header + property::Deserialize,
        <T as property::Header>::Id: BlockId,
        <T as property::Header>::Date: BlockDate,
    {
    }

    pub trait Block: property::Block + property::Deserialize {}

    impl<T> Block for T
    where
        T: property::Block + property::Deserialize,
        <T as property::Block>::Id: BlockId,
        <T as property::Block>::Date: BlockDate,
    {
    }
}

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
        P: Service<(), Response = S, Error = io::Error> + 'static,
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

pub struct ResponseFuture<T, R> {
    state: unary_future::State<T, R>,
}

impl<T, R> ResponseFuture<T, R> {
    fn new(future: GrpcFuture<R>) -> Self {
        ResponseFuture {
            state: unary_future::State::Pending(future),
        }
    }
}

pub struct ResponseStreamFuture<T, R> {
    state: stream_future::State<T, R>,
}

impl<T, R> ResponseStreamFuture<T, R> {
    fn new(future: GrpcStreamFuture<R>) -> Self {
        ResponseStreamFuture {
            state: stream_future::State::Pending(future),
        }
    }
}

pub struct ResponseStream<T, R> {
    inner: Streaming<R, tower_h2::RecvBody>,
    _phantom: PhantomData<T>,
}

fn convert_error<T>(e: tower_grpc::Error<T>) -> core_client::Error
where
    T: Debug + Send + Sync + 'static,
{
    core_client::Error::new(core_client::ErrorKind::Rpc, e)
}

pub trait FromResponse<T>: Sized {
    fn from_response(response: T) -> Result<Self, core_client::Error>;
}

mod unary_future {
    use super::{convert_error, core_client, FromResponse, GrpcError, GrpcFuture, ResponseFuture};
    use futures::prelude::*;
    use std::marker::PhantomData;
    use tower_grpc::Response;

    fn poll_and_convert_response<T, R, F>(future: &mut F) -> Poll<T, core_client::Error>
    where
        F: Future<Item = Response<R>, Error = GrpcError>,
        T: FromResponse<R>,
    {
        match future.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(res)) => {
                let item = T::from_response(res.into_inner())?;
                Ok(Async::Ready(item))
            }
            Err(e) => Err(convert_error(e)),
        }
    }

    pub enum State<T, R> {
        Pending(GrpcFuture<R>),
        Finished(PhantomData<T>),
    }

    impl<T, R> Future for ResponseFuture<T, R>
    where
        R: prost::Message + Default,
        T: FromResponse<R>,
    {
        type Item = T;
        type Error = core_client::Error;

        fn poll(&mut self) -> Poll<T, core_client::Error> {
            if let State::Pending(ref mut f) = self.state {
                let res = poll_and_convert_response(f);
                if let Ok(Async::NotReady) = res {
                    return Ok(Async::NotReady);
                }
                self.state = State::Finished(PhantomData);
                res
            } else {
                match self.state {
                    State::Pending(_) => unreachable!(),
                    State::Finished(_) => panic!("polled a finished response"),
                }
            }
        }
    }
}

mod stream_future {
    use super::{
        convert_error, core_client, GrpcError, GrpcStreamFuture, ResponseStream,
        ResponseStreamFuture,
    };
    use futures::prelude::*;
    use std::marker::PhantomData;
    use tower_grpc::{Response, Streaming};

    fn poll_and_convert_response<T, R, F>(
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

    pub enum State<T, R> {
        Pending(GrpcStreamFuture<R>),
        Finished(PhantomData<T>),
    }

    impl<T, R> Future for ResponseStreamFuture<T, R>
    where
        R: prost::Message + Default,
    {
        type Item = ResponseStream<T, R>;
        type Error = core_client::Error;

        fn poll(&mut self) -> Poll<ResponseStream<T, R>, core_client::Error> {
            if let State::Pending(ref mut f) = self.state {
                let res = poll_and_convert_response(f);
                if let Ok(Async::NotReady) = res {
                    return Ok(Async::NotReady);
                }
                self.state = State::Finished(PhantomData);
                res
            } else {
                match self.state {
                    State::Pending(_) => unreachable!(),
                    State::Finished(_) => panic!("polled a finished response"),
                }
            }
        }
    }
}

mod stream {
    use super::{convert_error, core_client, FromResponse, GrpcStreamError, ResponseStream};
    use futures::prelude::*;

    fn poll_and_convert_item<T, S, R>(stream: &mut S) -> Poll<Option<T>, core_client::Error>
    where
        S: Stream<Item = R, Error = GrpcStreamError>,
        T: FromResponse<R>,
    {
        match stream.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(item))) => {
                let item = T::from_response(item)?;
                Ok(Async::Ready(Some(item)))
            }
            Err(e) => Err(convert_error(e)),
        }
    }

    impl<T, R> Stream for ResponseStream<T, R>
    where
        R: prost::Message + Default,
        T: FromResponse<R>,
    {
        type Item = T;
        type Error = core_client::Error;

        fn poll(&mut self) -> Poll<Option<T>, core_client::Error> {
            poll_and_convert_item(&mut self.inner)
        }
    }
}

fn deserialize_bytes<T>(mut buf: &[u8]) -> Result<T, core_client::Error>
where
    T: Deserialize,
{
    T::deserialize(&mut buf).map_err(|e| core_client::Error::new(core_client::ErrorKind::Format, e))
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

impl<H> FromResponse<gen::node::TipResponse> for H
where
    H: chain_bounds::Header,
{
    fn from_response(res: gen::node::TipResponse) -> Result<Self, core_client::Error> {
        let blockheader = deserialize_bytes(&res.blockheader)?;
        Ok(blockheader)
    }
}

impl<T> FromResponse<gen::node::Block> for T
where
    T: chain_bounds::Block,
{
    fn from_response(res: gen::node::Block) -> Result<T, core_client::Error> {
        let block = deserialize_bytes(&res.content)?;
        Ok(block)
    }
}

impl<T> FromResponse<gen::node::Header> for T
where
    T: chain_bounds::Header,
{
    fn from_response(res: gen::node::Header) -> Result<T, core_client::Error> {
        let block = deserialize_bytes(&res.content)?;
        Ok(block)
    }
}

impl<T, S, E> BlockService<T> for Client<S, E>
where
    T: chain_bounds::Block + HasHeader,
    S: AsyncRead + AsyncWrite,
    E: Executor<Background<S, BoxBody>> + Clone,
    <T as property::Block>::Date: FromStr,
    <T::Header as property::Header>::Date: FromStr,
{
    type TipFuture = ResponseFuture<T::Header, gen::node::TipResponse>;

    type PullBlocksToTipStream = ResponseStream<T, gen::node::Block>;
    type PullBlocksToTipFuture = ResponseStreamFuture<T, gen::node::Block>;

    type GetBlocksStream = ResponseStream<T, gen::node::Block>;
    type GetBlocksFuture = ResponseStreamFuture<T, gen::node::Block>;

    fn tip(&mut self) -> Self::TipFuture {
        let req = gen::node::TipRequest {};
        let future = self.node.tip(Request::new(req));
        ResponseFuture::new(future)
    }

    fn pull_blocks_to_tip(&mut self, from: &[T::Id]) -> Self::PullBlocksToTipFuture {
        let from = serialize_to_vec(from);
        let req = gen::node::PullBlocksToTipRequest { from };
        let future = self.node.pull_blocks_to_tip(Request::new(req));
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
