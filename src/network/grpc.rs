use super::{ConnectionState, GlobalState};
use blockcfg::{Block, BlockHash, Header};
use intercom::{self, ClientMsg};
use settings::network::Listen;

use cardano as cardano_api;
use cardano::{
    block::EpochSlotId,
    hash,
    util::try_from_slice::TryFromSlice,
};

use futures::prelude::*;
use futures::{
    future::{self, FutureResult},
    sync::{mpsc, oneshot},
};
use tokio::{
    executor::DefaultExecutor,
    net::TcpListener,
};
use tower_grpc::{
    self,
    Request, Response,
};
use tower_h2::Server;

use std::net::SocketAddr;

// Included generated protobuf/gRPC code,
// namespaced into submodules corresponding to the .proto package names

mod cardano {
    include!(concat!(env!("OUT_DIR"), "/cardano.rs"));
}

#[allow(dead_code)]
mod iohk {
    pub mod jormungandr {
        include!(concat!(env!("OUT_DIR"), "/iohk.jormungandr.rs"));
    }
}

use self::iohk::jormungandr as gen;

// Conversions between library data types and their generated
// protobuf counterparts

fn try_hash_from_protobuf(
    pb: &cardano::HeaderHash
) -> Result<BlockHash, hash::Error> {
    BlockHash::try_from_slice(&pb.hash)
}

fn try_hashes_from_protobuf(
    pb: &cardano::HeaderHashes
) -> Result<Vec<BlockHash>, hash::Error> {
    pb.hashes.iter().map(|v| BlockHash::try_from_slice(&v[..])).collect()
}

impl From<BlockHash> for cardano::HeaderHash {
    fn from(hash: BlockHash) -> Self {
        cardano::HeaderHash {
            hash: hash.as_ref().into(),
        }
    }
}

impl From<Block> for cardano::Block {
    fn from(block: Block) -> Self {
        let content = cbor!(&block).unwrap();
        cardano::Block {
            content,
        }
    }
}

impl From<cardano_api::block::BlockDate> for cardano::BlockDate {
    fn from(date: cardano_api::block::BlockDate) -> Self {
        use self::cardano_api::block::BlockDate::*;
        let (epoch, slot) = match date {
            Boundary(epoch) => (epoch, 0),
            Normal(EpochSlotId { epoch, slotid }) => (epoch, slotid as u32),
        };
        cardano::BlockDate {
            epoch,
            slot
        }
    }
}

struct GrpcFuture<T> {
    receiver: oneshot::Receiver<Result<T, intercom::Error>>,
}

impl<T> Future for GrpcFuture<T> {
    type Item = Response<T>;
    type Error = tower_grpc::Error;

    fn poll(&mut self) -> Poll<Self::Item, tower_grpc::Error> {
        let item = match self.receiver.poll() {
            Err(oneshot::Canceled) => {
                warn!("gRPC response canceled by the client task");
                return Err(tower_grpc::Error::from(()));
            }
            Ok(Async::NotReady) => {
                return Ok(Async::NotReady);
            }
            Ok(Async::Ready(Err(e))) => {
                warn!("error processing gRPC request: {:?}", e);
                // FIXME: send a more informative error
                return Err(tower_grpc::Error::from(()));
            }
            Ok(Async::Ready(Ok(item))) => item,
        };

        Ok(Async::Ready(Response::new(item)))
    }
}

type ReplySender<T> = oneshot::Sender<Result<T, intercom::Error>>;

#[derive(Debug)]
struct ReplyHandle<T> {
    sender: Option<ReplySender<T>>,
}

impl<T> ReplyHandle<T> {
    fn take_sender(&mut self) -> ReplySender<T> {
        self.sender.take().unwrap()
    }
}

impl intercom::Reply<Header> for ReplyHandle<gen::TipResponse> {
    fn reply_ok(&mut self, header: Header) {
        let response = gen::TipResponse {
            blockdate: Some(header.get_blockdate().into()),
            hash: Some(header.compute_hash().into()),
        };
        self.take_sender().send(Ok(response)).unwrap();
    }

    fn reply_error(&mut self, error: intercom::Error) {
        self.take_sender().send(Err(error)).unwrap();
    }
}

struct GrpcResponseStream<T> {
    receiver: mpsc::UnboundedReceiver<Result<T, intercom::Error>>,
}

impl<T> Stream for GrpcResponseStream<T> {
    type Item = T;
    type Error = tower_grpc::Error;

    fn poll(&mut self) -> Poll<Option<T>, tower_grpc::Error> {
        match try_ready!(self.receiver.poll()) {
            None => Ok(Async::Ready(None)),
            Some(Ok(item)) => Ok(Async::Ready(Some(item))),
            // FIXME: send a more informative error
            Some(Err(_)) => Err(tower_grpc::Error::from(())),
        }
    }
}

#[derive(Debug)]
struct StreamReplyHandle<T> {
    sender: mpsc::UnboundedSender<Result<T, intercom::Error>>,
}

impl intercom::StreamReply<Block>
    for StreamReplyHandle<cardano::Block>
{
    fn send(&mut self, item: Block) {
        self.sender.unbounded_send(Ok(item.into())).unwrap()
    }

    fn send_error(&mut self, error: intercom::Error) {
        self.sender.unbounded_send(Err(error)).unwrap()
    }

    fn close(&mut self) {
        self.sender.close().unwrap();
    }
}

fn unary_response_channel<T>() -> (ReplyHandle<T>, GrpcFuture<T>) {
    let (sender, receiver) = oneshot::channel();
    (ReplyHandle { sender: Some(sender) }, GrpcFuture { receiver })
}

fn server_streaming_response_channel<T>(
) -> (StreamReplyHandle<T>, GrpcResponseStream<T>) {
    let (sender, receiver) = mpsc::unbounded();
    (StreamReplyHandle { sender }, GrpcResponseStream { receiver })
}

#[derive(Clone)]
struct GrpcServer {
    state: ConnectionState,
}

impl gen::server::Node for GrpcServer {
    type TipFuture = GrpcFuture<gen::TipResponse>;
    type GetBlocksStream = GrpcResponseStream<cardano::Block>;
    type GetBlocksFuture = FutureResult<
        Response<Self::GetBlocksStream>, tower_grpc::Error
    >;
    type GetHeadersStream = GrpcResponseStream<cardano::Header>;
    type GetHeadersFuture = FutureResult<
        Response<Self::GetHeadersStream>, tower_grpc::Error
    >;
    type StreamBlocksToTipStream = GrpcResponseStream<cardano::Block>;
    type StreamBlocksToTipFuture = FutureResult<
        Response<Self::StreamBlocksToTipStream>, tower_grpc::Error
    >;
    type ProposeTransactionsFuture = GrpcFuture<gen::ProposeTransactionsResponse>;
    type RecordTransactionFuture = GrpcFuture<gen::RecordTransactionResponse>;

    fn tip(&mut self, _request: Request<gen::TipRequest>) -> Self::TipFuture {
        let (handle, future) = unary_response_channel();
        self.state.channels.client_box.send_to(
            ClientMsg::GetBlockTip(Box::new(handle))
        );
        future
    }

    fn get_blocks(
        &mut self,
        _request: Request<gen::GetBlocksRequest>,
    ) -> Self::GetBlocksFuture {
        unimplemented!()
    }

    fn get_headers(
        &mut self,
        _request: Request<gen::GetBlocksRequest>,
    ) -> Self::GetHeadersFuture {
        unimplemented!()
    }

    fn stream_blocks_to_tip(
        &mut self,
        _from: Request<cardano::HeaderHashes>,
    ) -> Self::StreamBlocksToTipFuture {
        unimplemented!()
    }

    fn propose_transactions(
        &mut self,
        _request: Request<gen::ProposeTransactionsRequest>
    ) -> Self::ProposeTransactionsFuture {
        unimplemented!()
    }

    fn record_transaction(
        &mut self,
        _request: Request<gen::RecordTransactionRequest>
    ) -> Self::RecordTransactionFuture {
        unimplemented!()
    }
}

pub fn run_listen_socket(sockaddr: SocketAddr, listen: Listen, state: GlobalState)
    -> tokio::executor::Spawn
{
    let state = ConnectionState::new_listen(&state, listen);

    info!("start listening and accepting gRPC connections on {}", sockaddr);

    let node_service = gen::server::NodeServer::new(GrpcServer { state });

    let h2 = Server::new(
        node_service,
        Default::default(),
        DefaultExecutor::current(),
    );

    let server = TcpListener::bind(&sockaddr)
        .unwrap() // TODO, handle on error to provide better error message
        .incoming()
        .map_err(move |err| {
            // error while receiving an incoming connection
            // here we might need to log the error and try
            // to listen again on the sockaddr
            error!("Error while accepting connection on {}: {:?}", sockaddr, err);
        }).fold(h2, |mut h2, stream| {
            // received incoming connection
            info!(
                "{} connected to {}",
                stream.peer_addr().unwrap(),
                stream.local_addr().unwrap()
            );

            stream.set_nodelay(true).unwrap_or_else(|err| {
                error!(
                    "failed to set TCP_NODELAY on connection from {}: {:?}",
                    stream.peer_addr().unwrap(),
                    err,
                );
            });

            let serve = h2.serve(stream);

            tokio::spawn(serve.map_err(|e| error!("h2 error: {:?}", e)));

            Ok(h2)
        }).map(|_| {});

    tokio::spawn(server)
}
