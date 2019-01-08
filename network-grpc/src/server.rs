use chain_core::property::{Block, Deserialize, Serialize};

use futures::prelude::*;
use futures::try_ready;
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
    F: Future,
    T: From<<F as Future>::Item>,
    tower_grpc::Error: From<<F as Future>::Error>,
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

impl<T, F> Future for FutureResponse<T, F>
where
    F: Future,
    T: From<<F as Future>::Item>,
    tower_grpc::Error: From<<F as Future>::Error>,
{
    type Item = tower_grpc::Response<T>;
    type Error = tower_grpc::Error;

    fn poll(&mut self) -> Poll<Self::Item, tower_grpc::Error> {
        match self.state {
            ResponseState::Future(ref mut f) => {
                let item = try_ready!(f.poll());
                let response = tower_grpc::Response::new(item.into());
                Ok(Async::Ready(response))
            }
            ResponseState::Err(ref status) => Err(tower_grpc::Error::Grpc(status.clone())),
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
    T: From<<S as Stream>::Item>,
    tower_grpc::Error: From<<S as Stream>::Error>,
{
    pub fn new(stream: S) -> Self {
        ResponseStream {
            inner: stream,
            _phantom: PhantomData,
        }
    }
}

impl<T, S> From<S> for ResponseStream<T, S>
where
    S: Stream,
    T: From<<S as Stream>::Item>,
    tower_grpc::Error: From<<S as Stream>::Error>,
{
    fn from(stream: S) -> Self {
        ResponseStream::new(stream)
    }
}

impl<T, S> Stream for ResponseStream<T, S>
where
    S: Stream,
    T: From<<S as Stream>::Item>,
    tower_grpc::Error: From<<S as Stream>::Error>,
{
    type Item = T;
    type Error = tower_grpc::Error;

    fn poll(&mut self) -> Poll<Option<T>, tower_grpc::Error> {
        let maybe_item = try_ready!(self.inner.poll());
        Ok(Async::Ready(maybe_item.map(|item| item.into())))
    }
}

#[derive(Clone)]
pub struct GrpcServer<T> {
    node: T,
}

fn deserialize_hashes<H: Deserialize>(
    pb: &cardano_proto::HeaderHashes,
) -> Result<Vec<H>, H::Error> {
    pb.hashes
        .iter()
        .map(|v| H::deserialize(&mut &v[..]))
        .collect()
}

impl<T> gen::server::Node for GrpcServer<T>
where
    T: network_core::Node + Clone,
    tower_grpc::Error: From<<T as network_core::Node>::Error>,
    cardano_proto::Block: From<<T as network_core::Node>::Block>,
    cardano_proto::Header: From<<T as network_core::Node>::Header>,
    gen::TipResponse: From<<T as network_core::Node>::BlockId>,
    gen::ProposeTransactionsResponse:
        From<network_core::ProposeTransactionsResponse<<T as network_core::Node>::BlockId>>,
    gen::RecordTransactionResponse:
        From<network_core::RecordTransactionResponse<<T as network_core::Node>::BlockId>>,
{
    type TipFuture = FutureResponse<gen::TipResponse, <T as network_core::Node>::TipFuture>;
    type GetBlocksStream =
        ResponseStream<cardano_proto::Block, <T as network_core::Node>::BlocksStream>;
    type GetBlocksFuture =
        FutureResponse<Self::GetBlocksStream, <T as network_core::Node>::BlocksFuture>;
    type GetHeadersStream =
        ResponseStream<cardano_proto::Header, <T as network_core::Node>::HeadersStream>;
    type GetHeadersFuture =
        FutureResponse<Self::GetHeadersStream, <T as network_core::Node>::HeadersFuture>;
    type StreamBlocksToTipStream =
        ResponseStream<cardano_proto::Block, <T as network_core::Node>::BlocksStream>;
    type StreamBlocksToTipFuture =
        FutureResponse<Self::StreamBlocksToTipStream, <T as network_core::Node>::BlocksFuture>;
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
        from: Request<cardano_proto::HeaderHashes>,
    ) -> Self::StreamBlocksToTipFuture {
        let hashes = match deserialize_hashes(from.get_ref()) {
            Ok(hashes) => hashes,
            Err(e) => {
                // FIXME: log the error
                // (preferably with tower facilities outside of this implementation)
                let status = Status::with_code_and_message(Code::InvalidArgument, format!("{}", e));
                return FutureResponse::error(status);
            }
        };
        FutureResponse::new(self.node.stream_blocks_to_tip(&hashes))
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
