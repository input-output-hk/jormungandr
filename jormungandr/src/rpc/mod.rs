use std::net::SocketAddr;

use jsonrpc_http_server::jsonrpc_core::{IoHandler, Value};
use jsonrpc_http_server::ServerBuilder;

pub struct Config {
    pub listen: SocketAddr,
    pub threads: usize,
}

pub async fn start_rpc_server(config: Config) {
    // it is initial dummy impl just for initialization rpc instance
    let mut io = IoHandler::default();
    io.add_method("dummy", |_| async { Ok(Value::Null) });

    let server = ServerBuilder::new(io)
        .threads(config.threads)
        .start_http(&config.listen)
        .unwrap();

    server.wait();
}
