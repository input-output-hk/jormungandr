use crate::gen;

use chain_core::property::{Block, Deserialize, Header, Serialize, Transaction};
use network_core::{
    gossip::{self, Gossip},
    server::{block::BlockService, gossip::GossipService, transaction::TransactionService, Node},
};

use futures::prelude::*;
use tower_grpc::{self, Code, Request, Status};

use std::{error, marker::PhantomData, mem};

pub struct NodeService<T: Node> {
    block_service: Option<T::BlockService>,
    tx_service: Option<T::TransactionService>,
    gossip_service: Option<T::GossipService>,
}

impl<T: Node> NodeService<T> {
    pub fn new(node: T) -> Self {
        NodeService {
            block_service: node.block_service(),
            tx_service: node.transaction_service(),
            gossip_service: node.gossip_service(),
        }
    }
}

impl<T> Clone for NodeService<T>
where
    T: Node,
    T::BlockService: Clone,
    T::TransactionService: Clone,
    T::GossipService: Clone,
{
    fn clone(&self) -> Self {
        NodeService {
            block_service: self.block_service.clone(),
            tx_service: self.tx_service.clone(),
            gossip_service: self.gossip_service.clone(),
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
    F::Item: IntoResponse<T>,
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
        ResponseFuture::Failed(Status::new(Code::Unimplemented, "not implemented"))
    }
}

fn convert_error<E: error::Error>(e: E) -> tower_grpc::Status {
    Status::new(Code::Unknown, format!("{}", e))
}

pub trait IntoResponse<T> {
    fn into_response(self) -> Result<T, tower_grpc::Status>;
}

fn poll_and_convert_response<T, F>(
    future: &mut F,
) -> Poll<tower_grpc::Response<T>, tower_grpc::Status>
where
    F: Future,
    F::Item: IntoResponse<T>,
    F::Error: error::Error,
{
    match future.poll() {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(res)) => {
            let item = res.into_response()?;
            let response = tower_grpc::Response::new(item);
            Ok(Async::Ready(response))
        }
        Err(e) => Err(convert_error(e)),
    }
}

fn poll_and_convert_stream<T, S>(stream: &mut S) -> Poll<Option<T>, tower_grpc::Status>
where
    S: Stream,
    S::Item: IntoResponse<T>,
    S::Error: error::Error,
{
    match stream.poll() {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
        Ok(Async::Ready(Some(item))) => {
            let item = item.into_response()?;
            Ok(Async::Ready(Some(item)))
        }
        Err(e) => Err(convert_error(e)),
    }
}

impl<T, F> Future for ResponseFuture<T, F>
where
    F: Future,
    F::Item: IntoResponse<T>,
    F::Error: error::Error,
{
    type Item = tower_grpc::Response<T>;
    type Error = tower_grpc::Status;

    fn poll(&mut self) -> Poll<Self::Item, tower_grpc::Status> {
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
                ResponseFuture::Failed(status) => Err(status),
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
    S::Item: IntoResponse<T>,
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
    S::Item: IntoResponse<T>,
    S::Error: error::Error,
{
    type Item = T;
    type Error = tower_grpc::Status;

    fn poll(&mut self) -> Poll<Option<T>, tower_grpc::Status> {
        poll_and_convert_stream(&mut self.inner)
    }
}

fn deserialize_bytes<H: Deserialize>(v: &Vec<u8>) -> Result<H, tower_grpc::Status> {
    match H::deserialize(&mut &v[..]) {
        Ok(v) => Ok(v),
        Err(e) => {
            // FIXME: log the error
            // (preferably with tower facilities outside of this implementation)
            let status = Status::new(Code::InvalidArgument, format!("{}", e));
            Err(status)
        }
    }
}

fn deserialize_vec<H: Deserialize>(pb: &[Vec<u8>]) -> Result<Vec<H>, tower_grpc::Status> {
    match pb.iter().map(|v| H::deserialize(&mut &v[..])).collect() {
        Ok(v) => Ok(v),
        Err(e) => {
            // FIXME: log the error
            // (preferably with tower facilities outside of this implementation)
            let status = Status::new(Code::InvalidArgument, format!("{}", e));
            Err(status)
        }
    }
}

fn serialize_to_bytes<T>(obj: T) -> Result<Vec<u8>, tower_grpc::Status>
where
    T: Serialize,
{
    let mut bytes = Vec::new();
    match obj.serialize(&mut bytes) {
        Ok(()) => Ok(bytes),
        Err(_e) => {
            // FIXME: log the error
            let status = Status::new(Code::InvalidArgument, "response serialization failed");
            Err(status)
        }
    }
}

impl<S, T> IntoResponse<ResponseStream<T, S>> for S
where
    S: Stream,
    S::Item: IntoResponse<T>,
{
    fn into_response(self) -> Result<ResponseStream<T, S>, tower_grpc::Status> {
        let stream = ResponseStream::new(self);
        Ok(stream)
    }
}

impl<H> IntoResponse<gen::node::TipResponse> for H
where
    H: Header + Serialize,
{
    fn into_response(self) -> Result<gen::node::TipResponse, tower_grpc::Status> {
        let block_header = serialize_to_bytes(self)?;
        Ok(gen::node::TipResponse { block_header })
    }
}

impl<B> IntoResponse<gen::node::Block> for B
where
    B: Block + Serialize,
{
    fn into_response(self) -> Result<gen::node::Block, tower_grpc::Status> {
        let content = serialize_to_bytes(self)?;
        Ok(gen::node::Block { content })
    }
}

impl<G> IntoResponse<gen::node::GossipMessage> for (gossip::NodeId, G)
where
    G: Gossip + Serialize,
{
    fn into_response(self) -> Result<gen::node::GossipMessage, tower_grpc::Status> {
        let content = self.0.to_bytes();
        let node_id = Some(gen::node::gossip_message::NodeId { content });
        let content = serialize_to_bytes(self.1)?;
        Ok(gen::node::GossipMessage { node_id, content })
    }
}

impl<H> IntoResponse<gen::node::Header> for H
where
    H: Header + Serialize,
{
    fn into_response(self) -> Result<gen::node::Header, tower_grpc::Status> {
        let content = serialize_to_bytes(self)?;
        Ok(gen::node::Header { content })
    }
}

impl<T> IntoResponse<gen::node::Transaction> for T
where
    T: Transaction + Serialize,
{
    fn into_response(self) -> Result<gen::node::Transaction, tower_grpc::Status> {
        let content = serialize_to_bytes(self)?;
        Ok(gen::node::Transaction { content })
    }
}

macro_rules! try_get_service {
    ($opt_member:expr) => {
        match $opt_member {
            None => return ResponseFuture::unimplemented(),
            Some(ref mut service) => service,
        }
    };
}

impl<T> gen::node::server::Node for NodeService<T>
where
    T: Node,
    <T as Node>::BlockService: Clone,
    <T as Node>::TransactionService: Clone,
    <T as Node>::GossipService: Clone,
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
        <<T as Node>::BlockService as BlockService>::GetHeadersStream,
    >;
    type GetHeadersFuture = ResponseFuture<
        Self::GetHeadersStream,
        <<T as Node>::BlockService as BlockService>::GetHeadersFuture,
    >;
    type PullBlocksToTipStream = ResponseStream<
        gen::node::Block,
        <<T as Node>::BlockService as BlockService>::PullBlocksStream,
    >;
    type PullBlocksToTipFuture = ResponseFuture<
        Self::PullBlocksToTipStream,
        <<T as Node>::BlockService as BlockService>::PullBlocksFuture,
    >;
    type SubscribeToBlocksStream = ResponseStream<
        gen::node::Header,
        <<T as Node>::BlockService as BlockService>::BlockSubscription,
    >;
    type SubscribeToBlocksFuture = ResponseFuture<
        Self::SubscribeToBlocksStream,
        <<T as Node>::BlockService as BlockService>::BlockSubscriptionFuture,
    >;
    type TransactionsStream = ResponseStream<
        gen::node::Transaction,
        <<T as Node>::TransactionService as TransactionService>::GetTransactionsStream,
    >;
    type TransactionsFuture = ResponseFuture<
        Self::TransactionsStream,
        <<T as Node>::TransactionService as TransactionService>::GetTransactionsFuture,
    >;
    type GossipFuture = ResponseFuture<
        gen::node::GossipMessage,
        <<T as Node>::GossipService as GossipService>::MessageFuture,
    >;
    type AnnounceBlockFuture = ResponseFuture<
        gen::node::AnnounceBlockResponse,
        <<T as Node>::BlockService as BlockService>::AnnounceBlockFuture,
    >;
    type AnnounceTransactionFuture = ResponseFuture<
        gen::node::AnnounceTransactionResponse,
        <<T as Node>::TransactionService as TransactionService>::AnnounceTransactionFuture,
    >;

    fn tip(&mut self, _request: Request<gen::node::TipRequest>) -> Self::TipFuture {
        let service = try_get_service!(self.block_service);
        ResponseFuture::new(service.tip())
    }

    fn get_blocks(&mut self, req: Request<gen::node::BlockIds>) -> Self::GetBlocksFuture {
        let service = try_get_service!(self.block_service);
        let block_ids = match deserialize_vec(&req.get_ref().id) {
            Ok(block_ids) => block_ids,
            Err(status) => {
                return ResponseFuture::error(status);
            }
        };
        ResponseFuture::new(service.get_blocks(&block_ids))
    }

    fn get_headers(&mut self, req: Request<gen::node::BlockIds>) -> Self::GetHeadersFuture {
        let service = try_get_service!(self.block_service);
        let block_ids = match deserialize_vec(&req.get_ref().id) {
            Ok(block_ids) => block_ids,
            Err(status) => {
                return ResponseFuture::error(status);
            }
        };
        ResponseFuture::new(service.get_headers(&block_ids))
    }

    fn subscribe_to_blocks(
        &mut self,
        _req: Request<gen::node::BlockSubscriptionRequest>,
    ) -> Self::SubscribeToBlocksFuture {
        let service = try_get_service!(self.block_service);
        ResponseFuture::new(service.subscribe())
    }

    fn pull_blocks_to_tip(
        &mut self,
        req: Request<gen::node::PullBlocksToTipRequest>,
    ) -> Self::PullBlocksToTipFuture {
        let service = try_get_service!(self.block_service);
        let block_ids = match deserialize_vec(&req.get_ref().from) {
            Ok(block_ids) => block_ids,
            Err(status) => {
                return ResponseFuture::error(status);
            }
        };
        ResponseFuture::new(service.pull_blocks_to_tip(&block_ids))
    }

    fn transactions(
        &mut self,
        req: Request<gen::node::TransactionIds>,
    ) -> Self::TransactionsFuture {
        let service = try_get_service!(self.tx_service);
        let tx_ids = match deserialize_vec(&req.get_ref().id) {
            Ok(tx_ids) => tx_ids,
            Err(status) => {
                return ResponseFuture::error(status);
            }
        };
        ResponseFuture::new(service.get_transactions(&tx_ids))
    }

    fn announce_block(&mut self, req: Request<gen::node::Header>) -> Self::AnnounceBlockFuture {
        let service = try_get_service!(self.block_service);
        let header = match deserialize_bytes(&req.get_ref().content) {
            Ok(header) => header,
            Err(status) => {
                return ResponseFuture::error(status);
            }
        };
        ResponseFuture::new(service.announce_block(&header))
    }

    fn announce_transaction(
        &mut self,
        req: Request<gen::node::TransactionIds>,
    ) -> Self::AnnounceTransactionFuture {
        let service = try_get_service!(self.tx_service);
        let tx_ids = match deserialize_vec(&req.get_ref().id) {
            Ok(tx_ids) => tx_ids,
            Err(status) => {
                return ResponseFuture::error(status);
            }
        };
        ResponseFuture::new(service.announce_transaction(&tx_ids))
    }

    /// Work with gossip message.
    fn gossip(&mut self, req: Request<gen::node::GossipMessage>) -> Self::GossipFuture {
        let service = try_get_service!(self.gossip_service);
        let node_id = match &req.get_ref().node_id {
            Some(gen::node::gossip_message::NodeId { content }) => {
                match gossip::NodeId::from_slice(&content) {
                    Ok(node_id) => node_id,
                    Err(_v) => {
                        let status = Status::new(Code::InvalidArgument, "node decoding failed.");
                        return ResponseFuture::error(status);
                    }
                }
            }
            None => {
                let status = Status::new(Code::InvalidArgument, "node field is missing");
                return ResponseFuture::error(status);
            }
        };
        let gossip = match deserialize_bytes(&req.get_ref().content) {
            Ok(message) => message,
            Err(status) => {
                return ResponseFuture::error(status);
            }
        };
        ResponseFuture::new(service.record_gossip(node_id, &gossip))
    }
}
