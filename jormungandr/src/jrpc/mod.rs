#[cfg(feature = "evm")]
mod eth_block_info;
#[cfg(feature = "evm")]
mod eth_types;

use crate::context::ContextLock;
use jsonrpsee_http_server::{HttpServerBuilder, RpcModule};
use std::net::SocketAddr;

pub struct Config {
    pub listen: SocketAddr,
}

pub async fn start_jrpc_server(config: Config, _context: ContextLock) {
    let server = HttpServerBuilder::default()
        .build(config.listen)
        .await
        .unwrap();

    #[allow(unused_mut)]
    let mut modules = RpcModule::new(());

    #[cfg(feature = "evm")]
    modules
        .merge(eth_block_info::eth_get_blocks_info_module(_context))
        .unwrap();

    server.start(modules).unwrap().await
}
