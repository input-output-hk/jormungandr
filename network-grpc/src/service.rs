use crate::gen;

use chain_core::property::{
    Block, BlockDate, BlockId, Deserialize, Header, Serialize, TransactionId,
};
use network_core::server::{
    self,
    block::{BlockService, HeaderService},
    transaction::TransactionService,
    Node,
};

use futures::prelude::*;
use tower_grpc::Error::Grpc as GrpcError;
use tower_grpc::{self, Code, Request, Status};

use std::{error, marker::PhantomData, mem};

pub struct NodeService<T: Node> {
    block_service: Option<T::BlockService>,
    header_service: Option<T::HeaderService>,
    tx_service: Option<T::TransactionService>,
}

impl<T: Node> NodeService<T> {
    pub fn new(node: T) -> Self {
        NodeService {
            block_service: node.block_service(),
            header_service: node.header_service(),
            tx_service: node.transaction_service(),
        }
    }
}

impl<T> Clone for NodeService<T>
where
    T: Node,
    T::BlockService: Clone,
    T::HeaderService: Clone,
    T::TransactionService: Clone,
{
    fn clone(&self) -> Self {
        NodeService {
            block_service: self.block_service.clone(),
            header_service: self.header_service.clone(),
            tx_service: self.tx_service.clone(),
        }
    }
}

pub enum ResponseFuture<T, F> {
    Pending(F),
    Failed(Status),
    Finished(PhantomData<T>),
}

impl<T, F> ResponseFuture<T, F>
where
    F: Future,
    F::Item: ConvertResponse<T>,
{
    fn new(future: F) -> Self {
        ResponseFuture::Pending(future)
    }
}

impl<T, F> ResponseFuture<T, F> {
    fn error(status: Status) -> Self {
        ResponseFuture::Failed(status)
    }

    fn unimplemented() -> Self {
        ResponseFuture::Failed(Status::with_code(Code::Unimplemented))
    }
}

fn convert_error<E: error::Error>(e: E) -> tower_grpc::Error {
    let status = Status::with_code_and_message(Code::Unknown, format!("{}", e));
    GrpcError(status)
}

pub trait ConvertResponse<T> {
    fn convert_response(self) -> Result<T, tower_grpc::Error>;
}

fn poll_and_convert_response<T, F>(
    future: &mut F,
) -> Poll<tower_grpc::Response<T>, tower_grpc::Error>
where
    F: Future,
    F::Item: ConvertResponse<T>,
    F::Error: error::Error,
{
    match future.poll() {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(res)) => {
            let item = res.convert_response()?;
            let response = tower_grpc::Response::new(item);
            Ok(Async::Ready(response))
        }
        Err(e) => Err(convert_error(e)),
    }
}

fn poll_and_convert_stream<T, S>(stream: &mut S) -> Poll<Option<T>, tower_grpc::Error>
where
    S: Stream,
    S::Item: ConvertResponse<T>,
    S::Error: error::Error,
{
    match stream.poll() {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
        Ok(Async::Ready(Some(item))) => {
            let item = item.convert_response()?;
            Ok(Async::Ready(Some(item)))
        }
        Err(e) => Err(convert_error(e)),
    }
}

impl<T, F> Future for ResponseFuture<T, F>
where
    F: Future,
    F::Item: ConvertResponse<T>,
    F::Error: error::Error,
{
    type Item = tower_grpc::Response<T>;
    type Error = tower_grpc::Error;

    fn poll(&mut self) -> Poll<Self::Item, tower_grpc::Error> {
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
                ResponseFuture::Failed(status) => Err(GrpcError(status)),
                ResponseFuture::Finished(_) => panic!("polled a finished response"),
            }
        }
    }
}

pub struct ResponseStream<T, S> {
    inner: S,
    _phantom: PhantomData<T>,
}

impl<T, S> ResponseStream<T, S>
where
    S: Stream,
    S::Item: ConvertResponse<T>,
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
    S: Stream,
    S::Item: ConvertResponse<T>,
    S::Error: error::Error,
{
    type Item = T;
    type Error = tower_grpc::Error;

    fn poll(&mut self) -> Poll<Option<T>, tower_grpc::Error> {
        poll_and_convert_stream(&mut self.inner)
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

impl<S, T> ConvertResponse<ResponseStream<T, S>> for S
where
    S: Stream,
    S::Item: ConvertResponse<T>,
{
    fn convert_response(self) -> Result<ResponseStream<T, S>, tower_grpc::Error> {
        let stream = ResponseStream::new(self);
        Ok(stream)
    }
}

impl<I, D> ConvertResponse<gen::node::TipResponse> for (I, D)
where
    I: BlockId + Serialize,
    D: BlockDate + ToString,
{
    fn convert_response(self) -> Result<gen::node::TipResponse, tower_grpc::Error> {
        let id = serialize_to_bytes(self.0)?;
        let blockdate = self.1.to_string();
        let response = gen::node::TipResponse { id, blockdate };
        Ok(response)
    }
}

impl<B> ConvertResponse<gen::node::Block> for B
where
    B: Block + Serialize,
{
    fn convert_response(self) -> Result<gen::node::Block, tower_grpc::Error> {
        let content = serialize_to_bytes(self)?;
        Ok(gen::node::Block { content })
    }
}

impl<H> ConvertResponse<gen::node::Header> for H
where
    H: Header + Serialize,
{
    fn convert_response(self) -> Result<gen::node::Header, tower_grpc::Error> {
        let content = serialize_to_bytes(self)?;
        Ok(gen::node::Header { content })
    }
}

impl<I> ConvertResponse<gen::node::ProposeTransactionsResponse>
    for server::transaction::ProposeTransactionsResponse<I>
where
    I: TransactionId + Serialize,
{
    fn convert_response(self) -> Result<gen::node::ProposeTransactionsResponse, tower_grpc::Error> {
        unimplemented!();
    }
}

impl<I> ConvertResponse<gen::node::RecordTransactionResponse>
    for server::transaction::RecordTransactionResponse<I>
where
    I: TransactionId + Serialize,
{
    fn convert_response(self) -> Result<gen::node::RecordTransactionResponse, tower_grpc::Error> {
        unimplemented!();
    }
}

impl<T> gen::node::server::Node for NodeService<T>
where
    T: Node,
    <T as Node>::BlockService: Clone,
    <T as Node>::HeaderService: Clone,
    <T as Node>::TransactionService: Clone,
{
    type TipFuture = ResponseFuture<
        gen::node::TipResponse,
        <<T as Node>::BlockService as BlockService>::TipFuture,
    >;
    type GetBlocksStream = ResponseStream<
        gen::node::Block,
        <<T as Node>::BlockService as BlockService>::GetBlocksStream,
    >;
    type GetBlocksFuture = ResponseFuture<
        Self::GetBlocksStream,
        <<T as Node>::BlockService as BlockService>::GetBlocksFuture,
    >;
    type GetHeadersStream = ResponseStream<
        gen::node::Header,
        <<T as Node>::HeaderService as HeaderService>::GetHeadersStream,
    >;
    type GetHeadersFuture = ResponseFuture<
        Self::GetHeadersStream,
        <<T as Node>::HeaderService as HeaderService>::GetHeadersFuture,
    >;
    type StreamBlocksToTipStream = ResponseStream<
        gen::node::Block,
        <<T as Node>::BlockService as BlockService>::StreamBlocksToTipStream,
    >;
    type StreamBlocksToTipFuture = ResponseFuture<
        Self::StreamBlocksToTipStream,
        <<T as Node>::BlockService as BlockService>::StreamBlocksToTipFuture,
    >;
    type ProposeTransactionsFuture = ResponseFuture<
        gen::node::ProposeTransactionsResponse,
        <<T as Node>::TransactionService as TransactionService>::ProposeTransactionsFuture,
    >;
    type RecordTransactionFuture = ResponseFuture<
        gen::node::RecordTransactionResponse,
        <<T as Node>::TransactionService as TransactionService>::RecordTransactionFuture,
    >;

    fn tip(&mut self, _request: Request<gen::node::TipRequest>) -> Self::TipFuture {
        let service = match self.block_service {
            None => return ResponseFuture::unimplemented(),
            Some(ref mut service) => service,
        };
        ResponseFuture::new(service.tip())
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
            None => return ResponseFuture::unimplemented(),
            Some(ref mut service) => service,
        };
        let block_ids = match deserialize_vec(&req.get_ref().from) {
            Ok(block_ids) => block_ids,
            Err(GrpcError(status)) => {
                return ResponseFuture::error(status);
            }
            Err(e) => panic!("unexpected error {:?}", e),
        };
        ResponseFuture::new(service.stream_blocks_to_tip(&block_ids))
    }

    fn propose_transactions(
        &mut self,
        _request: Request<gen::node::ProposeTransactionsRequest>,
    ) -> Self::ProposeTransactionsFuture {
        let _service = match self.tx_service {
            None => return ResponseFuture::unimplemented(),
            Some(ref mut service) => service,
        };
        unimplemented!()
    }

    fn record_transaction(
        &mut self,
        _request: Request<gen::node::RecordTransactionRequest>,
    ) -> Self::RecordTransactionFuture {
        let _service = match self.tx_service {
            None => return ResponseFuture::unimplemented(),
            Some(ref mut service) => service,
        };
        unimplemented!()
    }
}
