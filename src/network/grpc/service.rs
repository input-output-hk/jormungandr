use network::{ConnectionState, GlobalState};
use blockcfg::{chain::cardano::{Block, BlockHash, Header}};
use intercom::{self, ClientMsg};
use settings::network::Listen;

use cardano::block::{BlockDate, EpochSlotId};

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

use super::cardano as cardano_proto;
use super::iohk::jormungandr as gen;
use super::try_hashes_from_protobuf;

impl From<BlockHash> for cardano_proto::HeaderHash {
    fn from(hash: BlockHash) -> Self {
        cardano_proto::HeaderHash {
            hash: hash.as_ref().into(),
        }
    }
}

impl From<Block> for cardano_proto::Block {
    fn from(block: Block) -> Self {
        let content = cbor!(&block).unwrap();
        cardano_proto::Block {
            content,
        }
    }
}

impl From<BlockDate> for cardano_proto::BlockDate {
    fn from(date: BlockDate) -> Self {
        use self::BlockDate::*;
        let (epoch, slot) = match date {
            Boundary(epoch) => (epoch, 0),
            Normal(EpochSlotId { epoch, slotid }) => (epoch, slotid as u32),
        };
        cardano_proto::BlockDate {
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
    for StreamReplyHandle<cardano_proto::Block>
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
    type GetBlocksStream = GrpcResponseStream<cardano_proto::Block>;
    type GetBlocksFuture = FutureResult<
        Response<Self::GetBlocksStream>, tower_grpc::Error
    >;
    type GetHeadersStream = GrpcResponseStream<cardano_proto::Header>;
    type GetHeadersFuture = FutureResult<
        Response<Self::GetHeadersStream>, tower_grpc::Error
    >;
    type StreamBlocksToTipStream = GrpcResponseStream<cardano_proto::Block>;
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
        from: Request<cardano_proto::HeaderHashes>,
    ) -> Self::StreamBlocksToTipFuture {
        let hashes = match try_hashes_from_protobuf(from.get_ref()) {
            Ok(hashes) => hashes,
            Err(e) => {
                // FIXME: send a more detailed error
                return future::err(tower_grpc::Error::from(()));
            }
        };
        let (handle, stream) = server_streaming_response_channel();
        self.state.channels.client_box.send_to(
            ClientMsg::StreamBlocksToTip(hashes, Box::new(handle))
        );
        future::ok(Response::new(stream))
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
