use crate::{blockchain::StorageError, context::ContextLock};
use jsonrpsee_http_server::RpcModule;

mod logic;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ContextError(#[from] crate::context::Error),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

pub fn eth_get_blocks_info_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_getBlockByHash", |params, context| async move {
            let context = context.read().await;
            let (block_hash, full) = params.parse()?;
            logic::get_block_by_hash(block_hash, full, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
}
