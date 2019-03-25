use crate::blockchain::BlockchainR;
use actix_web::{App, Responder, State};
use chain_core::property::Settings as _;

pub fn create_handler(
    blockchain: BlockchainR,
) -> impl Fn(&str) -> App<BlockchainR> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        App::with_state(blockchain.clone())
            .prefix(format!("{}/v0/tip", prefix))
            .resource("", |r| r.get().with(handle_request))
    }
}

fn handle_request(settings: State<BlockchainR>) -> impl Responder {
    settings.read().unwrap().state.tip().to_string()
}
