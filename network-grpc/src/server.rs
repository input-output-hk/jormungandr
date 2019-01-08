use chain_core::property::{Block, Deserialize, Serialize};

use futures::prelude::*;
use tower_grpc::Error::Grpc as GrpcError;
use tower_grpc::{self, Code, Request, Status};

use std::marker::PhantomData;

use super::cardano as cardano_proto;
use super::iohk::jormungandr as gen;
use super::network_core;

enum ResponseState<F> {
    Future(F),
    Err(Status),
}

pub struct FutureResponse<T, F> {
    state: ResponseState<F>,
    _phantom: PhantomData<T>,
}

impl<T, F> FutureResponse<T, F>
where
    F: Future + ConvertResponse<T>,
{
    fn new(future: F) -> Self {
        FutureResponse {
            state: ResponseState::Future(future),
            _phantom: PhantomData,
        }
    }
}

impl<T, F> FutureResponse<T, F> {
    fn error(status: Status) -> Self {
        FutureResponse {
            state: ResponseState::Err(status),
            _phantom: PhantomData,
        }
    }
}

fn convert_error(e: network_core::Error) -> tower_grpc::Error {
    let status = Status::with_code_and_message(Code::Unknown, format!("{}", e));
    GrpcError(status)
}

pub trait ConvertResponse<T>: Future<Error = network_core::Error> {
    fn convert_item(item: Self::Item) -> Result<T, tower_grpc::Error>;
}

pub trait ConvertStream<T>: Stream<Error = network_core::Error> {
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
        match self.state {
            ResponseState::Future(ref mut f) => poll_and_convert_response(f),
            ResponseState::Err(ref status) => Err(GrpcError(status.clone())),
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

fn deserialize_block_ids<H: Deserialize>(pb: &cardano_proto::BlockIds) -> Result<Vec<H>, H::Error> {
    pb.ids.iter().map(|v| H::deserialize(&mut &v[..])).collect()
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
    F: Future<Item = S, Error = network_core::Error>,
    S: Stream + ConvertStream<T>,
{
    fn convert_item(item: S) -> Result<ResponseStream<T, S>, tower_grpc::Error> {
        let stream = ResponseStream::new(item);
        Ok(stream)
    }
}

impl<F, I, D> ConvertResponse<gen::TipResponse> for F
where
    F: Future<Item = (I, D), Error = network_core::Error>,
    I: Serialize,
    D: Serialize,
{
    fn convert_item(item: (I, D)) -> Result<gen::TipResponse, tower_grpc::Error> {
        let id = serialize_to_bytes(item.0)?;
        let blockdate = serialize_to_bytes(item.1)?;
        let response = gen::TipResponse {
            id: Some(cardano_proto::BlockId { id }),
            blockdate: Some(cardano_proto::BlockDate { content: blockdate }),
        };
        Ok(response)
    }
}

impl<S, B> ConvertStream<cardano_proto::Block> for S
where
    S: Stream<Item = B, Error = network_core::Error>,
    B: Block + Serialize,
{
    fn convert_item(item: Self::Item) -> Result<cardano_proto::Block, tower_grpc::Error> {
        let content = serialize_to_bytes(item)?;
        Ok(cardano_proto::Block { content })
    }
}

impl<S, H> ConvertStream<cardano_proto::Header> for S
where
    S: Stream<Item = H, Error = network_core::Error>,
    H: Serialize, // FIXME: this needs more bounds to only work for headers
{
    fn convert_item(item: Self::Item) -> Result<cardano_proto::Header, tower_grpc::Error> {
        let content = serialize_to_bytes(item)?;
        Ok(cardano_proto::Header { content })
    }
}

impl<F, I> ConvertResponse<gen::ProposeTransactionsResponse> for F
where
    F: Future<Item = network_core::ProposeTransactionsResponse<I>, Error = network_core::Error>,
    I: Serialize,
{
    fn convert_item(
        _item: Self::Item,
    ) -> Result<gen::ProposeTransactionsResponse, tower_grpc::Error> {
        unimplemented!();
    }
}

impl<F, I> ConvertResponse<gen::RecordTransactionResponse> for F
where
    F: Future<Item = network_core::RecordTransactionResponse<I>, Error = network_core::Error>,
    I: Serialize,
{
    fn convert_item(
        _item: Self::Item,
    ) -> Result<gen::RecordTransactionResponse, tower_grpc::Error> {
        unimplemented!();
    }
}

impl<T> gen::server::Node for GrpcServer<T>
where
    T: network_core::Node + Clone,
    <T as network_core::Node>::BlockId: Serialize + Deserialize,
    <T as network_core::Node>::BlockDate: Serialize,
    <T as network_core::Node>::Header: Serialize,
{
    type TipFuture = FutureResponse<gen::TipResponse, <T as network_core::Node>::TipFuture>;
    type GetBlocksStream =
        ResponseStream<cardano_proto::Block, <T as network_core::Node>::GetBlocksStream>;
    type GetBlocksFuture =
        FutureResponse<Self::GetBlocksStream, <T as network_core::Node>::GetBlocksFuture>;
    type GetHeadersStream =
        ResponseStream<cardano_proto::Header, <T as network_core::Node>::GetHeadersStream>;
    type GetHeadersFuture =
        FutureResponse<Self::GetHeadersStream, <T as network_core::Node>::GetHeadersFuture>;
    type StreamBlocksToTipStream =
        ResponseStream<cardano_proto::Block, <T as network_core::Node>::StreamBlocksToTipStream>;
    type StreamBlocksToTipFuture = FutureResponse<
        Self::StreamBlocksToTipStream,
        <T as network_core::Node>::StreamBlocksToTipFuture,
    >;
    type ProposeTransactionsFuture = FutureResponse<
        gen::ProposeTransactionsResponse,
        <T as network_core::Node>::ProposeTransactionsFuture,
    >;
    type RecordTransactionFuture = FutureResponse<
        gen::RecordTransactionResponse,
        <T as network_core::Node>::RecordTransactionFuture,
    >;

    fn tip(&mut self, _request: Request<gen::TipRequest>) -> Self::TipFuture {
        FutureResponse::new(self.node.tip())
    }

    fn get_blocks(&mut self, _request: Request<gen::GetBlocksRequest>) -> Self::GetBlocksFuture {
        unimplemented!()
    }

    fn get_headers(&mut self, _request: Request<gen::GetBlocksRequest>) -> Self::GetHeadersFuture {
        unimplemented!()
    }

    fn stream_blocks_to_tip(
        &mut self,
        from: Request<cardano_proto::BlockIds>,
    ) -> Self::StreamBlocksToTipFuture {
        let block_ids = match deserialize_block_ids(from.get_ref()) {
            Ok(block_ids) => block_ids,
            Err(e) => {
                // FIXME: log the error
                // (preferably with tower facilities outside of this implementation)
                let status = Status::with_code_and_message(Code::InvalidArgument, format!("{}", e));
                return FutureResponse::error(status);
            }
        };
        FutureResponse::new(self.node.stream_blocks_to_tip(&block_ids))
    }

    fn propose_transactions(
        &mut self,
        _request: Request<gen::ProposeTransactionsRequest>,
    ) -> Self::ProposeTransactionsFuture {
        unimplemented!()
    }

    fn record_transaction(
        &mut self,
        _request: Request<gen::RecordTransactionRequest>,
    ) -> Self::RecordTransactionFuture {
        unimplemented!()
    }
}
