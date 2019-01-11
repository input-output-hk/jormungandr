use chain_core::property::{Block, BlockDate, BlockId, Deserialize, Header, Serialize};
use network_core::server;

use futures::prelude::*;
use tower_grpc::Error::Grpc as GrpcError;
use tower_grpc::{self, Code, Request, Status};

use std::marker::PhantomData;

use super::gen;

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
}

fn convert_error(e: server::Error) -> tower_grpc::Error {
    let status = Status::with_code_and_message(Code::Unknown, format!("{}", e));
    GrpcError(status)
}

pub trait ConvertResponse<T>: Future<Error = server::Error> {
    fn convert_item(item: Self::Item) -> Result<T, tower_grpc::Error>;
}

pub trait ConvertStream<T>: Stream<Error = server::Error> {
    fn convert_item(item: Self::Item) -> Result<T, tower_grpc::Error>;
}

fn poll_and_convert_response<T, F>(
    future: &mut F,
) -> Poll<tower_grpc::Response<T>, tower_grpc::Error>
where
    F: Future + ConvertResponse<T>,
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
{
    type Item = T;
    type Error = tower_grpc::Error;

    fn poll(&mut self) -> Poll<Option<T>, tower_grpc::Error> {
        poll_and_convert_stream(&mut self.inner)
    }
}

#[derive(Clone)]
pub struct GrpcServer<T> {
    node: T,
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
    F: Future<Item = S, Error = server::Error>,
    S: Stream + ConvertStream<T>,
{
    fn convert_item(item: S) -> Result<ResponseStream<T, S>, tower_grpc::Error> {
        let stream = ResponseStream::new(item);
        Ok(stream)
    }
}

impl<F, I, D> ConvertResponse<gen::node::TipResponse> for F
where
    F: Future<Item = (I, D), Error = server::Error>,
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
    S: Stream<Item = B, Error = server::Error>,
    B: Block + Serialize,
{
    fn convert_item(item: Self::Item) -> Result<gen::node::Block, tower_grpc::Error> {
        let content = serialize_to_bytes(item)?;
        Ok(gen::node::Block { content })
    }
}

impl<S, H> ConvertStream<gen::node::Header> for S
where
    S: Stream<Item = H, Error = server::Error>,
    H: Header + Serialize,
{
    fn convert_item(item: Self::Item) -> Result<gen::node::Header, tower_grpc::Error> {
        let content = serialize_to_bytes(item)?;
        Ok(gen::node::Header { content })
    }
}

impl<F, I> ConvertResponse<gen::node::ProposeTransactionsResponse> for F
where
    F: Future<Item = server::ProposeTransactionsResponse<I>, Error = server::Error>,
    I: BlockId + Serialize,
{
    fn convert_item(
        _item: Self::Item,
    ) -> Result<gen::node::ProposeTransactionsResponse, tower_grpc::Error> {
        unimplemented!();
    }
}

impl<F, I> ConvertResponse<gen::node::RecordTransactionResponse> for F
where
    F: Future<Item = server::RecordTransactionResponse<I>, Error = server::Error>,
    I: BlockId + Serialize,
{
    fn convert_item(
        _item: Self::Item,
    ) -> Result<gen::node::RecordTransactionResponse, tower_grpc::Error> {
        unimplemented!();
    }
}

impl<T> gen::node::server::Node for GrpcServer<T>
where
    T: server::Node + Clone,
    <T as server::Node>::BlockId: Serialize + Deserialize,
    <T as server::Node>::BlockDate: ToString,
    <T as server::Node>::Header: Serialize,
{
    type TipFuture = FutureResponse<gen::node::TipResponse, <T as server::Node>::TipFuture>;
    type GetBlocksStream = ResponseStream<gen::node::Block, <T as server::Node>::GetBlocksStream>;
    type GetBlocksFuture =
        FutureResponse<Self::GetBlocksStream, <T as server::Node>::GetBlocksFuture>;
    type GetHeadersStream =
        ResponseStream<gen::node::Header, <T as server::Node>::GetHeadersStream>;
    type GetHeadersFuture =
        FutureResponse<Self::GetHeadersStream, <T as server::Node>::GetHeadersFuture>;
    type StreamBlocksToTipStream =
        ResponseStream<gen::node::Block, <T as server::Node>::StreamBlocksToTipStream>;
    type StreamBlocksToTipFuture =
        FutureResponse<Self::StreamBlocksToTipStream, <T as server::Node>::StreamBlocksToTipFuture>;
    type ProposeTransactionsFuture = FutureResponse<
        gen::node::ProposeTransactionsResponse,
        <T as server::Node>::ProposeTransactionsFuture,
    >;
    type RecordTransactionFuture = FutureResponse<
        gen::node::RecordTransactionResponse,
        <T as server::Node>::RecordTransactionFuture,
    >;

    fn tip(&mut self, _request: Request<gen::node::TipRequest>) -> Self::TipFuture {
        FutureResponse::new(self.node.tip())
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
        let block_ids = match deserialize_vec(&req.get_ref().from) {
            Ok(block_ids) => block_ids,
            Err(GrpcError(status)) => {
                return FutureResponse::error(status);
            }
            Err(e) => panic!("unexpected error {:?}", e),
        };
        FutureResponse::new(self.node.stream_blocks_to_tip(&block_ids))
    }

    fn propose_transactions(
        &mut self,
        _request: Request<gen::node::ProposeTransactionsRequest>,
    ) -> Self::ProposeTransactionsFuture {
        unimplemented!()
    }

    fn record_transaction(
        &mut self,
        _request: Request<gen::node::RecordTransactionRequest>,
    ) -> Self::RecordTransactionFuture {
        unimplemented!()
    }
}
