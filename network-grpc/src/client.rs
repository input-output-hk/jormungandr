use crate::{
    convert::serialize_to_vec,
    gen::{self, node::client as gen_client},
};

use chain_core::property;
use network_core::{
    client::{block::BlockService, gossip::GossipService},
    error as core_error,
    gossip::{Gossip, Node},
};

use futures::future::Executor;
use tokio::io;
use tokio::prelude::*;
use tower::MakeService;
use tower_add_origin::{self, AddOrigin};
use tower_grpc::{codegen::server::tower::Service, BoxBody, Request, Streaming};
use tower_h2::client::{Background, Connect, ConnectError, Connection};

use std::{error, fmt, marker::PhantomData};

/// Traits setting additional bounds for blockchain entities
/// that need to be satisfied for the protocol implementation.
///
/// The traits are auto-implemented for the types that satisfy the necessary
/// bounds. These traits then can be used in lieu of the lengthy bound clauses,
/// so that, should the implementation requrements change, only these trait
/// definitions and blanket implementations need to be modified.
pub mod chain_bounds {
    use chain_core::property;

    pub trait BlockId: property::BlockId + property::Deserialize
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

    pub trait Block: property::Block + property::HasHeader + property::Deserialize {}

    impl<T> Block for T
    where
        T: property::Block + property::HasHeader + property::Deserialize,
        <T as property::Block>::Id: BlockId,
        <T as property::Block>::Date: BlockDate,
        <T as property::HasHeader>::Header: Header,
    {
    }
}

/// A trait that fixes the types of protocol entities and the bounds
/// these entities need to satisfy for the protocol implementation.
pub trait ProtocolConfig {
    type BlockId: chain_bounds::BlockId;
    type BlockDate: chain_bounds::BlockDate;
    type Header: chain_bounds::Header + property::Header<Id = Self::BlockId, Date = Self::BlockDate>;
    type Block: chain_bounds::Block
        + property::Block<Id = Self::BlockId, Date = Self::BlockDate>
        + property::HasHeader<Header = Self::Header>;
    type Node: Node;
}

/// gRPC client for blockchain node.
///
/// This type encapsulates the gRPC protocol client that can
/// make connections and perform requests towards other blockchain nodes.
pub struct Client<C, S, E>
where
    C: ProtocolConfig,
{
    node: gen_client::Node<AddOrigin<Connection<S, E, BoxBody>>>,
    _phantom: PhantomData<(C::Block)>,
}

impl<C, S, E> Client<C, S, E>
where
    C: ProtocolConfig,
    S: AsyncRead + AsyncWrite,
    E: Executor<Background<S, BoxBody>> + Clone,
{
    pub fn connect<P>(
        peer: P,
        executor: E,
        uri: http::Uri,
    ) -> impl Future<Item = Self, Error = Error>
    where
        P: Service<(), Response = S, Error = io::Error> + 'static,
    {
        let mut make_client = Connect::new(peer, Default::default(), executor);
        make_client
            .make_service(())
            .map_err(|e| Error::Connect(e))
            .map(|conn| {
                let conn = tower_add_origin::Builder::new()
                    .uri(uri)
                    .build(conn)
                    .unwrap();
                Client {
                    node: gen_client::Node::new(conn),
                    _phantom: PhantomData,
                }
            })
    }
}

type GrpcFuture<R> = tower_grpc::client::unary::ResponseFuture<
    R,
    tower_h2::client::ResponseFuture,
    tower_h2::RecvBody,
>;

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

pub struct ResponseStreamFuture<T, F> {
    state: stream_future::State<T, F>,
}

impl<T, F> ResponseStreamFuture<T, F> {
    fn new(future: F) -> Self {
        ResponseStreamFuture {
            state: stream_future::State::Pending(future),
        }
    }
}

pub type ServerStreamFuture<T, R> = ResponseStreamFuture<
    T,
    tower_grpc::client::server_streaming::ResponseFuture<R, tower_h2::client::ResponseFuture>,
>;

pub type BidiStreamFuture<T, R> = ResponseStreamFuture<
    T,
    tower_grpc::client::streaming::ResponseFuture<R, tower_h2::client::ResponseFuture>,
>;

pub struct ResponseStream<T, R> {
    inner: Streaming<R, tower_h2::RecvBody>,
    _phantom: PhantomData<T>,
}

mod unary_future {
    use super::{core_error, GrpcFuture, ResponseFuture};
    use crate::convert::{error_from_grpc, FromProtobuf};
    use futures::prelude::*;
    use std::marker::PhantomData;
    use tower_grpc::{Response, Status};

    fn poll_and_convert_response<T, R, F>(future: &mut F) -> Poll<T, core_error::Error>
    where
        T: FromProtobuf<R>,
        F: Future<Item = Response<R>, Error = Status>,
    {
        match future.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(res)) => {
                let item = T::from_message(res.into_inner())?;
                Ok(Async::Ready(item))
            }
            Err(e) => Err(error_from_grpc(e)),
        }
    }

    pub enum State<T, R> {
        Pending(GrpcFuture<R>),
        Finished(PhantomData<T>),
    }

    impl<T, R> Future for ResponseFuture<T, R>
    where
        R: prost::Message + Default,
        T: FromProtobuf<R>,
    {
        type Item = T;
        type Error = core_error::Error;

        fn poll(&mut self) -> Poll<T, core_error::Error> {
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
    use super::{core_error, ResponseStream, ResponseStreamFuture};
    use crate::convert::error_from_grpc;
    use futures::prelude::*;
    use std::marker::PhantomData;
    use tower_grpc::{Response, Status, Streaming};

    fn poll_and_convert_response<T, R, F>(
        future: &mut F,
    ) -> Poll<ResponseStream<T, R>, core_error::Error>
    where
        F: Future<Item = Response<Streaming<R, tower_h2::RecvBody>>, Error = Status>,
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
            Err(e) => Err(error_from_grpc(e)),
        }
    }

    pub enum State<T, F> {
        Pending(F),
        Finished(PhantomData<T>),
    }

    impl<T, R, F> Future for ResponseStreamFuture<T, F>
    where
        F: Future<Item = Response<Streaming<R, tower_h2::RecvBody>>, Error = Status>,
    {
        type Item = ResponseStream<T, R>;
        type Error = core_error::Error;

        fn poll(&mut self) -> Poll<ResponseStream<T, R>, core_error::Error> {
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

mod response_stream {
    use super::{core_error, ResponseStream};
    use crate::convert::{error_from_grpc, FromProtobuf};
    use futures::prelude::*;
    use tower_grpc::Status;

    fn poll_and_convert_item<T, S, R>(stream: &mut S) -> Poll<Option<T>, core_error::Error>
    where
        S: Stream<Item = R, Error = Status>,
        T: FromProtobuf<R>,
    {
        match stream.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(item))) => {
                let item = T::from_message(item)?;
                Ok(Async::Ready(Some(item)))
            }
            Err(e) => Err(error_from_grpc(e)),
        }
    }

    impl<T, R> Stream for ResponseStream<T, R>
    where
        R: prost::Message + Default,
        T: FromProtobuf<R>,
    {
        type Item = T;
        type Error = core_error::Error;

        fn poll(&mut self) -> Poll<Option<T>, core_error::Error> {
            poll_and_convert_item(&mut self.inner)
        }
    }
}

pub struct RequestStream<S, R> {
    inner: S,
    _phantom: PhantomData<R>,
}

impl<S, R> RequestStream<S, R>
where
    S: Stream,
{
    fn new(inner: S) -> Self {
        RequestStream {
            inner,
            _phantom: PhantomData,
        }
    }
}

mod request_stream {
    use super::RequestStream;
    use crate::convert::IntoProtobuf;
    use futures::prelude::*;
    use tower_grpc::{Code, Status};

    fn poll_and_convert_item<S, R>(stream: &mut S) -> Poll<Option<R>, Status>
    where
        S: Stream,
        S::Item: IntoProtobuf<R>,
    {
        match stream.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(item))) => {
                let item = item.into_message()?;
                Ok(Async::Ready(Some(item)))
            }
            Err(_) => Err(Status::new(Code::Unknown, "request stream failure")),
        }
    }

    impl<S, R> Stream for RequestStream<S, R>
    where
        S: Stream,
        S::Item: IntoProtobuf<R>,
    {
        type Item = R;
        type Error = Status;

        fn poll(&mut self) -> Poll<Option<R>, Status> {
            poll_and_convert_item(&mut self.inner)
        }
    }
}

impl<C, S, E> BlockService for Client<C, S, E>
where
    C: ProtocolConfig,
    S: AsyncRead + AsyncWrite,
    E: Executor<Background<S, BoxBody>> + Clone,
{
    type Block = C::Block;
    type TipFuture = ResponseFuture<C::Header, gen::node::TipResponse>;

    type PullBlocksToTipStream = ResponseStream<C::Block, gen::node::Block>;
    type PullBlocksToTipFuture = ServerStreamFuture<C::Block, gen::node::Block>;

    type GetBlocksStream = ResponseStream<C::Block, gen::node::Block>;
    type GetBlocksFuture = ServerStreamFuture<C::Block, gen::node::Block>;

    type BlockSubscription = ResponseStream<C::Header, gen::node::Header>;
    type BlockSubscriptionFuture = BidiStreamFuture<C::Header, gen::node::Header>;

    fn tip(&mut self) -> Self::TipFuture {
        let req = gen::node::TipRequest {};
        let future = self.node.tip(Request::new(req));
        ResponseFuture::new(future)
    }

    fn pull_blocks_to_tip(&mut self, from: &[C::BlockId]) -> Self::PullBlocksToTipFuture {
        let from = serialize_to_vec(from).unwrap();
        let req = gen::node::PullBlocksToTipRequest { from };
        let future = self.node.pull_blocks_to_tip(Request::new(req));
        ServerStreamFuture::new(future)
    }

    fn block_subscription<Out>(&mut self, outbound: Out) -> Self::BlockSubscriptionFuture
    where
        Out: Stream<Item = C::Header> + Send + 'static,
    {
        let req = RequestStream::new(outbound);
        let future = self.node.block_subscription(Request::new(req));
        BidiStreamFuture::new(future)
    }
}

impl<C, S, E> GossipService for Client<C, S, E>
where
    C: ProtocolConfig,
    S: AsyncRead + AsyncWrite,
    E: Executor<Background<S, BoxBody>> + Clone,
{
    type Node = C::Node;
    type GossipSubscription = ResponseStream<Gossip<C::Node>, gen::node::Gossip>;
    type GossipSubscriptionFuture = BidiStreamFuture<Gossip<C::Node>, gen::node::Gossip>;

    fn gossip_subscription<Out>(&mut self, outbound: Out) -> Self::GossipSubscriptionFuture
    where
        Out: Stream<Item = Gossip<C::Node>> + Send + 'static,
    {
        let req = RequestStream::new(outbound);
        let future = self.node.gossip_subscription(Request::new(req));
        BidiStreamFuture::new(future)
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
            Error::Connect(_) => write!(f, "failed to establish connection"),
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
