use super::{ConnectionState, GlobalState};
use blockcfg::{BlockHash, Header};
use intercom::{self, ClientMsg};
use settings::network::Listen;

use cardano as cardano_api;
use cardano::block::EpochSlotId;

use futures::prelude::*;
use futures::{
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

mod iohk {
    pub mod jormungandr {
        include!(concat!(env!("OUT_DIR"), "/iohk.jormungandr.rs"));
    }
}

use self::iohk::jormungandr as gen;

// Conversions from library data types to their generated
// protobuf counterparts

impl From<BlockHash> for cardano::HeaderHash {
    fn from(hash: BlockHash) -> Self {
        cardano::HeaderHash {
            hash: hash.as_ref().into(),
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
    receiver: mpsc::UnboundedReceiver<T>,
}

impl<T> Stream for GrpcResponseStream<T> {
    type Item = T;
    type Error = tower_grpc::Error;

    fn poll(&mut self) -> Poll<Option<T>, tower_grpc::Error> {
        let item_or_eof = try_ready!(self.receiver.poll());
        Ok(Async::Ready(item_or_eof))
    }
}

fn response_channel<T>() -> (ReplyHandle<T>, GrpcFuture<T>) {
    let (sender, receiver) = oneshot::channel();
    (ReplyHandle { sender: Some(sender) }, GrpcFuture { receiver })
}

#[derive(Clone)]
struct GrpcServer {
    state: ConnectionState,
}

impl gen::server::Node for GrpcServer {
    type TipFuture = GrpcFuture<gen::TipResponse>;
    type GetBlocksStream = GrpcResponseStream<cardano::Block>;
    type GetBlocksFuture = GrpcFuture<Self::GetBlocksStream>;

    fn tip(&mut self, _request: Request<gen::TipRequest>) -> Self::TipFuture {
        let (handle, future) = response_channel();
        self.state.channels.client_box.send_to(
            ClientMsg::GetBlockTip(Box::new(handle))
        );
        future
    }

    fn get_blocks(
        &mut self,
        request: Request<gen::GetBlocksRequest>,
    ) -> Self::GetBlocksFuture {
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
