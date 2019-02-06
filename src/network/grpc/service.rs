use crate::blockcfg::{
    cardano::{Block, BlockHash},
    BlockConfig, Deserialize,
};
use crate::intercom::{self, ClientMsg};
use crate::network::{ConnectionState, GlobalState};
use crate::settings::network::Listen;

use cardano::block::{BlockDate, EpochSlotId};
use chain_core::property;

use futures::prelude::*;
use futures::{
    future::{self, FutureResult},
    sync::{mpsc, oneshot},
};
use tokio::{executor::DefaultExecutor, net::TcpListener};

use std::net::SocketAddr;

struct GrpcServer<B: BlockConfig> {
    state: ConnectionState<B>,
}

impl<B: BlockConfig> Clone for GrpcServer<B> {
    fn clone(&self) -> Self {
        GrpcServer {
            state: self.state.clone(),
        }
    }
}

impl<B> gen::server::Node for GrpcServer<B>
where
    B: BlockConfig,
    <B as BlockConfig>::Block: Into<cardano_proto::Block>,
    <B as BlockConfig>::BlockHash: Into<cardano_proto::HeaderHash> + Deserialize,
    <B as BlockConfig>::BlockDate: Into<cardano_proto::BlockDate>,
{
    type TipFuture = GrpcFuture<gen::TipResponse>;
    type GetBlocksStream = GrpcResponseStream<cardano_proto::Block>;
    type GetBlocksFuture = FutureResult<Response<Self::GetBlocksStream>, tower_grpc::Error>;
    type GetHeadersStream = GrpcResponseStream<cardano_proto::Header>;
    type GetHeadersFuture = FutureResult<Response<Self::GetHeadersStream>, tower_grpc::Error>;
    type StreamBlocksToTipStream = GrpcResponseStream<cardano_proto::Block>;
    type StreamBlocksToTipFuture =
        FutureResult<Response<Self::StreamBlocksToTipStream>, tower_grpc::Error>;
    type ProposeTransactionsFuture = GrpcFuture<gen::ProposeTransactionsResponse>;
    type RecordTransactionFuture = GrpcFuture<gen::RecordTransactionResponse>;

    fn tip(&mut self, _request: Request<gen::TipRequest>) -> Self::TipFuture {
        let (handle, future) = unary_response_channel();
        self.state
            .channels
            .client_box
            .send_to(ClientMsg::GetBlockTip(Box::new(handle)));
        future
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
                info!(
                    "failed to decode hashes from StreamBlocksToTip request: {:?}",
                    e
                );
                let status = Status::with_code_and_message(Code::InvalidArgument, format!("{}", e));
                return future::err(tower_grpc::Error::Grpc(status));
            }
        };
        let (handle, stream) = server_streaming_response_channel();
        self.state
            .channels
            .client_box
            .send_to(ClientMsg::StreamBlocksToTip(hashes, Box::new(handle)));
        future::ok(Response::new(stream))
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

pub fn run_listen_socket<B>(
    sockaddr: SocketAddr,
    listen: Listen,
    state: GlobalState<B>,
) -> tokio::executor::Spawn
where
    B: 'static + BlockConfig,
    <B as BlockConfig>::Block: Into<cardano_proto::Block> + Send,
    <B as BlockConfig>::BlockHash: Into<cardano_proto::HeaderHash> + Deserialize + Send,
    <B as BlockConfig>::BlockDate: Into<cardano_proto::BlockDate>,
    <B as BlockConfig>::Transaction: Send,
    <B as BlockConfig>::TransactionId: Send,
{
    let state = ConnectionState::new_listen(&state, listen);

    info!(
        "start listening and accepting gRPC connections on {}",
        sockaddr
    );

    let node_service = gen::server::NodeServer::new(GrpcServer { state });

    let h2 = Server::new(node_service, Default::default(), DefaultExecutor::current());

    let server = TcpListener::bind(&sockaddr)
        .unwrap() // TODO, handle on error to provide better error message
        .incoming()
        .map_err(move |err| {
            // error while receiving an incoming connection
            // here we might need to log the error and try
            // to listen again on the sockaddr
            error!(
                "Error while accepting connection on {}: {:?}",
                sockaddr, err
            );
        })
        .fold(h2, |mut h2, stream| {
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
        })
        .map(|_| {});

    tokio::spawn(server)
}
