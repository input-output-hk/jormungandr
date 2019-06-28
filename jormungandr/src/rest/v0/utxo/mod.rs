use actix_web::App;
use blockchain::BlockchainR;

use crate::rest::v0::handlers;

pub fn create_handler(
    blockchain: BlockchainR,
) -> impl Fn(&str) -> App<BlockchainR> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        let app_prefix = format!("{}/v0/utxo", prefix);
        App::with_state(blockchain.clone())
            .prefix(app_prefix)
            .resource("", |r| r.get().with(handlers::get_utxos))
    }
}