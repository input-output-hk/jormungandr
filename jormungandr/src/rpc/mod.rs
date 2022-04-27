mod eth_block_info;

use crate::context::ContextLock;
use jsonrpsee_http_server::{HttpServerBuilder, RpcModule};
use std::net::SocketAddr;

pub struct Config {
    pub listen: SocketAddr,
}

pub async fn start_rpc_server(config: Config, context: ContextLock) {
    // it is initial dummy impl just for initialization rpc instance
    let server = HttpServerBuilder::default()
        .build(config.listen)
        .await
        .unwrap();

    let mut modules = RpcModule::new(());

    modules
        .merge(eth_block_info::eth_get_blocks_info_module())
        .unwrap();

    server.start(modules).unwrap().await
}
