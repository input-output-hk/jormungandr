use std::net::SocketAddr;

// use jsonrpsee::jsonrpc_core::{IoHandler, Value};
// use jsonrpsee::ServerBuilder;
use jsonrpsee_http_server::{HttpServerBuilder, RpcModule};

pub struct Config {
    pub listen: SocketAddr,
}

pub async fn start_rpc_server(config: Config) {
    // it is initial dummy impl just for initialization rpc instance
    let server = HttpServerBuilder::default()
        .build(config.listen)
        .await
        .unwrap();

    let mut module = RpcModule::new(());
    module.register_method("dummy", |_, _| Ok(())).unwrap();

    server.start(module).unwrap().await
}
