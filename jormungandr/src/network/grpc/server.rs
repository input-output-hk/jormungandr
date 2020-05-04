use super::super::{service::NodeService, Channels, GlobalStateR, ListenError};
use crate::settings::start::network::Listen;
use chain_network::grpc;

use tonic::transport::Server;

pub async fn run_listen_socket(
    listen: &Listen,
    state: GlobalStateR,
    channels: Channels,
) -> Result<(), ListenError> {
    let sockaddr = listen.address();

    let logger = state.logger().new(o!("local_addr" => sockaddr.to_string()));
    info!(logger, "listening and accepting gRPC connections");

    let service = grpc::Server::new(grpc::NodeService::new(NodeService::new(channels, state)));

    Server::builder()
        .add_service(service)
        .serve(sockaddr)
        .await
        .map_err(|cause| ListenError { cause, sockaddr })
}
