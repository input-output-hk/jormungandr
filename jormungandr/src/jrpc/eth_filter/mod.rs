use crate::context::ContextLock;
use jsonrpsee_http_server::RpcModule;

mod logic;

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub fn eth_filter_module(context: ContextLock) -> RpcModule<ContextLock> {
    let module = RpcModule::new(context);

    module
}
