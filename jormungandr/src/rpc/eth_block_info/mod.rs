use jsonrpsee_http_server::RpcModule;

pub fn eth_get_blocks_info_module() -> RpcModule<()> {
    let mut module = RpcModule::new(());

    module
        .register_method("eth_getBlockByHash", |_, _| Ok(()))
        .unwrap();

    module
}
