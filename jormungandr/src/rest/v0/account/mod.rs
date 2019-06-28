use crate::blockchain::BlockchainR;
use actix_web::App;

use crate::rest::v0::handlers;

pub fn create_handler(
    blockchain: BlockchainR,
) -> impl Fn(&str) -> App<BlockchainR> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        App::with_state(blockchain.clone())
            .prefix(format!("{}/v0/account", prefix))
            .resource("/{account_id}", |r| r.get().with(handlers::get_account_state))
    }
}
