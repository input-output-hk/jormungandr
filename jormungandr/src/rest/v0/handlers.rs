use crate::rest::{v0::logic, ContextLock};
use warp::{reject::Reject, Rejection, Reply};

impl Reject for logic::Error {}

pub async fn get_account_state(
    account_id_hex: String,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_account_state(&context, &account_id_hex)
        .await
        .map_err(warp::reject::custom)?
        .map(|r| warp::reply::json(&r))
        .ok_or_else(warp::reject::not_found)
}

pub async fn get_message_logs(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_message_logs(&context)
        .await
        .map_err(warp::reject::custom)
        .map(|r| warp::reply::json(&r))
}

pub async fn post_message(
    message: warp::hyper::body::Bytes,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::post_message(&context, &message)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_tip(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_tip(&context).await.map_err(warp::reject::custom)
}

pub async fn get_stats_counter(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_stats_counter(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_block_id(
    block_id_hex: String,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_block_id(&context, &block_id_hex)
        .await
        .map_err(warp::reject::custom)?
        .ok_or_else(warp::reject::not_found)
}

#[derive(Deserialize)]
pub struct GetBlockNextIdQuery {
    count: Option<u32>,
}

pub async fn get_block_next_id(
    block_id_hex: String,
    query: GetBlockNextIdQuery,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    let count = query.count.unwrap_or(1);
    logic::get_block_next_id(&context, &block_id_hex, count as usize)
        .await
        .map_err(warp::reject::custom)?
        .ok_or_else(warp::reject::not_found)
}

pub async fn get_stake_distribution(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_stake_distribution(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_stake_distribution_at(
    epoch: u32,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_stake_distribution_at(&context, epoch)
        .await
        .map_err(warp::reject::custom)?
        .map(|r| warp::reply::json(&r))
        .ok_or_else(warp::reject::not_found)
}

pub async fn get_settings(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_settings(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn shutdown(context: ContextLock) -> Result<impl Reply, Rejection> {
    let mut context = context.write().await;
    logic::shutdown(&mut context)
        .await
        .map(|_| warp::reply())
        .map_err(warp::reject::custom)
}

pub async fn get_leaders_logs(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_leaders_logs(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_stake_pools(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_stake_pools(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_stats(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_network_stats(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_rewards_info_epoch(
    epoch: u32,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_rewards_info_epoch(&context, epoch)
        .await
        .map_err(warp::reject::custom)?
        .map(|r| warp::reply::json(&r))
        .ok_or_else(warp::reject::not_found)
}

pub async fn get_rewards_info_history(
    length: usize,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_rewards_info_history(&context, length)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_rewards_remaining(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_rewards_remaining(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_utxo(
    fragment_id_hex: String,
    output_index: u8,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_utxo(&context, &fragment_id_hex, output_index)
        .await
        .map_err(warp::reject::custom)?
        .map(|r| warp::reply::json(&r))
        .ok_or_else(warp::reject::not_found)
}

pub async fn get_stake_pool(
    pool_id_hex: String,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_stake_pool(&context, &pool_id_hex)
        .await
        .map_err(warp::reject::custom)?
        .map(|r| warp::reply::json(&r))
        .ok_or_else(warp::reject::not_found)
}

pub async fn get_diagnostic(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_diagnostic(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_p2p_quarantined(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_network_p2p_quarantined(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_p2p_non_public(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_network_p2p_non_public(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_p2p_available(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_network_p2p_available(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_p2p_view(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_network_p2p_view(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_p2p_view_topic(
    topic: String,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_network_p2p_view_topic(&context, &topic)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_committees(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_committees(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_active_vote_plans(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_active_vote_plans(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

#[cfg(feature = "evm")]
pub async fn get_jor_address(
    evm_id_hex: String,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_jor_address(&context, &evm_id_hex)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

#[cfg(feature = "evm")]
pub async fn get_evm_address(
    evm_id_hex: String,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_evm_address(&context, &evm_id_hex)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}
