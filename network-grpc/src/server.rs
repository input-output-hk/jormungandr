use chain_core::property::{
    Block, BlockDate, BlockId, Deserialize, Header, Serialize, TransactionId,
};
use network_core::server::{self, block::BlockService, transaction::TransactionService, Node};

use futures::prelude::*;
use tokio::{
    net::{TcpListener, TcpStream},
    runtime::current_thread::TaskExecutor,
};
use tower_grpc::Error::Grpc as GrpcError;
use tower_grpc::{self, Code, Request, Status};

use std::{error::Error as ErrorTrait, fmt, marker::PhantomData, net::SocketAddr};

use crate::gen;

pub enum FutureResponse<T, F> {
    Pending(F),
    Err(Status),
    Complete(PhantomData<T>),
}

impl<T, F> FutureResponse<T, F>
where
    F: Future + ConvertResponse<T>,
{
    fn new(future: F) -> Self {
        FutureResponse::Pending(future)
    }
}

impl<T, F> FutureResponse<T, F> {
    fn error(status: Status) -> Self {
        FutureResponse::Err(status)
    }

    fn unimplemented() -> Self {
        FutureResponse::Err(Status::with_code(Code::Unimplemented))
    }
}

fn convert_error<E: ErrorTrait>(e: E) -> tower_grpc::Error {
    let status = Status::with_code_and_message(Code::Unknown, format!("{}", e));
    GrpcError(status)
}

pub trait ConvertResponse<T>: Future {
    fn convert_item(item: Self::Item) -> Result<T, tower_grpc::Error>;
}

pub trait ConvertStream<T>: Stream {
    fn convert_item(item: Self::Item) -> Result<T, tower_grpc::Error>;
}

fn poll_and_convert_response<T, F>(
    future: &mut F,
) -> Poll<tower_grpc::Response<T>, tower_grpc::Error>
where
    F: Future + ConvertResponse<T>,
    F::Error: ErrorTrait,
{
    match future.poll() {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(item)) => {
            let item = F::convert_item(item)?;
            let response = tower_grpc::Response::new(item);
            Ok(Async::Ready(response))
        }
        Err(e) => Err(convert_error(e)),
    }
}

fn poll_and_convert_stream<T, S>(stream: &mut S) -> Poll<Option<T>, tower_grpc::Error>
where
    S: Stream + ConvertStream<T>,
    S::Error: ErrorTrait,
{
    match stream.poll() {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
        Ok(Async::Ready(Some(item))) => {
            let item = S::convert_item(item)?;
            Ok(Async::Ready(Some(item)))
        }
        Err(e) => Err(convert_error(e)),
    }
}

impl<T, F> Future for FutureResponse<T, F>
where
    F: Future + ConvertResponse<T>,
    F::Error: ErrorTrait,
{
    type Item = tower_grpc::Response<T>;
    type Error = tower_grpc::Error;

    fn poll(&mut self) -> Poll<Self::Item, tower_grpc::Error> {
        let res = match self {
            FutureResponse::Pending(f) => poll_and_convert_response(f),
            FutureResponse::Err(status) => Err(GrpcError(status.clone())),
            FutureResponse::Complete(_) => panic!("polled a finished response"),
        };
        if let Ok(Async::NotReady) = res {
            Ok(Async::NotReady)
        } else {
            *self = FutureResponse::Complete(PhantomData);
            res
        }
    }
}

pub struct ResponseStream<T, S> {
    inner: S,
    _phantom: PhantomData<T>,
}

impl<T, S> ResponseStream<T, S>
where
    S: Stream + ConvertStream<T>,
{
    pub fn new(stream: S) -> Self {
        ResponseStream {
            inner: stream,
            _phantom: PhantomData,
        }
    }
}

impl<T, S> Stream for ResponseStream<T, S>
where
    S: Stream + ConvertStream<T>,
    S::Error: ErrorTrait,
{
    type Item = T;
    type Error = tower_grpc::Error;

    fn poll(&mut self) -> Poll<Option<T>, tower_grpc::Error> {
        poll_and_convert_stream(&mut self.inner)
    }
}

struct NodeService<T: Node> {
    block_service: Option<T::BlockService>,
    tx_service: Option<T::TransactionService>,
}

impl<T> Clone for NodeService<T>
where
    T: Node,
    T::BlockService: Clone,
    T::TransactionService: Clone,
{
    fn clone(&self) -> Self {
        NodeService {
            block_service: self.block_service.clone(),
            tx_service: self.tx_service.clone(),
        }
    }
}

/// The gRPC server for the blockchain node.
///
/// This type encapsulates the gRPC protocol server providing the
/// Node service. The application instantiates a `Server` wrapping a
/// blockchain service implementation satisfying the abstract network
/// service trait `Node`.
pub struct Server<T>
where
    T: Node,
    <T as Node>::BlockService: Clone,
    <T as Node>::TransactionService: Clone,
{
    h2: tower_h2::Server<
        gen::node::server::NodeServer<NodeService<T>>,
        TaskExecutor,
        gen::node::server::node::ResponseBody<NodeService<T>>,
    >,
}

/// Connection of a client peer to the gRPC server.
pub struct Connection<T>(
    tower_h2::server::Connection<
        TcpStream,
        gen::node::server::NodeServer<NodeService<T>>,
        TaskExecutor,
        gen::node::server::node::ResponseBody<NodeService<T>>,
        (),
    >,
)
where
    T: Node,
    <T as Node>::BlockService: Clone,
    <T as Node>::TransactionService: Clone;

impl<T> Server<T>
where
    T: Node + 'static,
    <T as Node>::BlockService: Clone,
    <T as Node>::TransactionService: Clone,
{
    /// Creates a server instance around the node service implementation.
    pub fn new(node: T) -> Self {
        let grpc_service = gen::node::server::NodeServer::new(NodeService {
            block_service: node.block_service(),
            tx_service: node.transaction_service(),
        });
        let h2 = tower_h2::Server::new(grpc_service, Default::default(), TaskExecutor::current());
        Server { h2 }
    }

    /// Initializes a client peer connection based on an accepted TCP
    /// connection socket.
    /// The socket can be obtained from a stream returned by `listen`.
    pub fn serve(&mut self, sock: TcpStream) -> Connection<T> {
        Connection(self.h2.serve(sock))
    }
}

/// Sets up a listening TCP socket bound to the given address.
/// If successful, returns an asynchronous stream of `TcpStream` socket
/// objects representing accepted TCP connections from clients.
/// The TCP_NODELAY option is disabled on the returned sockets as
/// necessary for the HTTP/2 protocol.
pub fn listen(addr: &SocketAddr) -> Result<impl Stream<Item = TcpStream, Error = Error>, Error> {
    let listener = TcpListener::bind(&addr)?;
    let stream = listener
        .incoming()
        .and_then(|sock| {
            sock.set_nodelay(true)?;
            Ok(sock)
        })
        .map_err(|e| Error::Io(e));
    Ok(stream)
}

/// The error type for gRPC server operations.
#[derive(Debug)]
pub enum Error {
    Io(tokio::io::Error),
}

impl From<tokio::io::Error> for Error {
    fn from(err: tokio::io::Error) -> Self {
        Error::Io(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl ErrorTrait for Error {
    fn source(&self) -> Option<&(dyn ErrorTrait + 'static)> {
        match self {
            Error::Io(e) => Some(e),
        }
    }
}

fn deserialize_vec<H: Deserialize>(pb: &[Vec<u8>]) -> Result<Vec<H>, tower_grpc::Error> {
    match pb.iter().map(|v| H::deserialize(&mut &v[..])).collect() {
        Ok(v) => Ok(v),
        Err(e) => {
            // FIXME: log the error
            // (preferably with tower facilities outside of this implementation)
            let status = Status::with_code_and_message(Code::InvalidArgument, format!("{}", e));
            Err(GrpcError(status))
        }
    }
}

fn serialize_to_bytes<T>(obj: T) -> Result<Vec<u8>, tower_grpc::Error>
where
    T: Serialize,
{
    let mut bytes = Vec::new();
    match obj.serialize(&mut bytes) {
        Ok(()) => Ok(bytes),
        Err(_e) => {
            // FIXME: log the error
            let status = Status::with_code(Code::Unknown);
            Err(GrpcError(status))
        }
    }
}

impl<F, S, T> ConvertResponse<ResponseStream<T, S>> for F
where
    F: Future<Item = S>,
    S: Stream + ConvertStream<T>,
{
    fn convert_item(item: S) -> Result<ResponseStream<T, S>, tower_grpc::Error> {
        let stream = ResponseStream::new(item);
        Ok(stream)
    }
}

impl<F, I, D> ConvertResponse<gen::node::TipResponse> for F
where
    F: Future<Item = (I, D)>,
    I: BlockId + Serialize,
    D: BlockDate + ToString,
{
    fn convert_item(item: (I, D)) -> Result<gen::node::TipResponse, tower_grpc::Error> {
        let id = serialize_to_bytes(item.0)?;
        let blockdate = item.1.to_string();
        let response = gen::node::TipResponse { id, blockdate };
        Ok(response)
    }
}

impl<S, B> ConvertStream<gen::node::Block> for S
where
    S: Stream<Item = B>,
    B: Block + Serialize,
{
    fn convert_item(item: Self::Item) -> Result<gen::node::Block, tower_grpc::Error> {
        let content = serialize_to_bytes(item)?;
        Ok(gen::node::Block { content })
    }
}

impl<S, H> ConvertStream<gen::node::Header> for S
where
    S: Stream<Item = H>,
    H: Header + Serialize,
{
    fn convert_item(item: Self::Item) -> Result<gen::node::Header, tower_grpc::Error> {
        let content = serialize_to_bytes(item)?;
        Ok(gen::node::Header { content })
    }
}

impl<F, I> ConvertResponse<gen::node::ProposeTransactionsResponse> for F
where
    F: Future<Item = server::transaction::ProposeTransactionsResponse<I>>,
    I: TransactionId + Serialize,
{
    fn convert_item(
        _item: Self::Item,
    ) -> Result<gen::node::ProposeTransactionsResponse, tower_grpc::Error> {
        unimplemented!();
    }
}

impl<F, I> ConvertResponse<gen::node::RecordTransactionResponse> for F
where
    F: Future<Item = server::transaction::RecordTransactionResponse<I>>,
    I: TransactionId + Serialize,
{
    fn convert_item(
        _item: Self::Item,
    ) -> Result<gen::node::RecordTransactionResponse, tower_grpc::Error> {
        unimplemented!();
    }
}

impl<T> gen::node::server::Node for NodeService<T>
where
    T: Node,
    <T as Node>::BlockService: Clone,
    <T as Node>::TransactionService: Clone,
{
    type TipFuture = FutureResponse<
        gen::node::TipResponse,
        <<T as Node>::BlockService as BlockService>::TipFuture,
    >;
    type GetBlocksStream = ResponseStream<
        gen::node::Block,
        <<T as Node>::BlockService as BlockService>::GetBlocksStream,
    >;
    type GetBlocksFuture = FutureResponse<
        Self::GetBlocksStream,
        <<T as Node>::BlockService as BlockService>::GetBlocksFuture,
    >;
    type GetHeadersStream = ResponseStream<
        gen::node::Header,
        <<T as Node>::BlockService as BlockService>::GetHeadersStream,
    >;
    type GetHeadersFuture = FutureResponse<
        Self::GetHeadersStream,
        <<T as Node>::BlockService as BlockService>::GetHeadersFuture,
    >;
    type StreamBlocksToTipStream = ResponseStream<
        gen::node::Block,
        <<T as Node>::BlockService as BlockService>::StreamBlocksToTipStream,
    >;
    type StreamBlocksToTipFuture = FutureResponse<
        Self::StreamBlocksToTipStream,
        <<T as Node>::BlockService as BlockService>::StreamBlocksToTipFuture,
    >;
    type ProposeTransactionsFuture = FutureResponse<
        gen::node::ProposeTransactionsResponse,
        <<T as Node>::TransactionService as TransactionService>::ProposeTransactionsFuture,
    >;
    type RecordTransactionFuture = FutureResponse<
        gen::node::RecordTransactionResponse,
        <<T as Node>::TransactionService as TransactionService>::RecordTransactionFuture,
    >;

    fn tip(&mut self, _request: Request<gen::node::TipRequest>) -> Self::TipFuture {
        let service = match self.block_service {
            None => return FutureResponse::unimplemented(),
            Some(ref mut service) => service,
        };
        FutureResponse::new(service.tip())
    }

    fn get_blocks(
        &mut self,
        _request: Request<gen::node::GetBlocksRequest>,
    ) -> Self::GetBlocksFuture {
        unimplemented!()
    }

    fn get_headers(
        &mut self,
        _request: Request<gen::node::GetBlocksRequest>,
    ) -> Self::GetHeadersFuture {
        unimplemented!()
    }

    fn stream_blocks_to_tip(
        &mut self,
        req: Request<gen::node::StreamBlocksToTipRequest>,
    ) -> Self::StreamBlocksToTipFuture {
        let service = match self.block_service {
            None => return FutureResponse::unimplemented(),
            Some(ref mut service) => service,
        };
        let block_ids = match deserialize_vec(&req.get_ref().from) {
            Ok(block_ids) => block_ids,
            Err(GrpcError(status)) => {
                return FutureResponse::error(status);
            }
            Err(e) => panic!("unexpected error {:?}", e),
        };
        FutureResponse::new(service.stream_blocks_to_tip(&block_ids))
    }

    fn propose_transactions(
        &mut self,
        _request: Request<gen::node::ProposeTransactionsRequest>,
    ) -> Self::ProposeTransactionsFuture {
        let _service = match self.tx_service {
            None => return FutureResponse::unimplemented(),
            Some(ref mut service) => service,
        };
        unimplemented!()
    }

    fn record_transaction(
        &mut self,
        _request: Request<gen::node::RecordTransactionRequest>,
    ) -> Self::RecordTransactionFuture {
        let _service = match self.tx_service {
            None => return FutureResponse::unimplemented(),
            Some(ref mut service) => service,
        };
        unimplemented!()
    }
}
