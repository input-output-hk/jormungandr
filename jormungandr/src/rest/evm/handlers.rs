use crate::{context::ContextLock, rest::evm::logic};
use warp::{reject::Reject, Rejection, Reply};

impl Reject for logic::Error {}

pub async fn get_jor_address(
    evm_id_hex: String,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_jor_address(&context, &evm_id_hex)
        .await
        .map_err(warp::reject::custom)
}

pub async fn get_evm_address(
    evm_id_hex: String,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_evm_address(&context, &evm_id_hex)
        .await
        .map_err(warp::reject::custom)
        .map(|r| warp::reply::json(&r))
}
