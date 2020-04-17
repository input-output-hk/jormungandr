use crate::{
    rest::{v0::logic, Context},
    secure::NodeSecret,
};

use warp::{reject::Reject, Rejection, Reply};

impl Reject for logic::Error {}

pub async fn get_account_state(
    account_id_hex: String,
    context: Context,
) -> Result<impl Reply, Rejection> {
    logic::get_account_state(&context, &account_id_hex)
        .await
        .map_err(warp::reject::custom)?
        .map(|r| warp::reply::json(&r))
        .ok_or(warp::reject::not_found())
}

pub async fn get_message_logs(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_message_logs(&context)
        .await
        .map_err(warp::reject::custom)
        .map(|r| warp::reply::json(&r))
}

pub async fn post_message(
    message: bytes::Bytes,
    context: Context,
) -> Result<impl Reply, Rejection> {
    logic::post_message(&context, &message)
        .await
        .map(|()| warp::reply())
        .map_err(warp::reject::custom)
}

pub async fn get_tip(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_tip(&context).await.map_err(warp::reject::custom)
}

pub async fn get_stats_counter(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_stats_counter(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_block_id(block_id_hex: String, context: Context) -> Result<impl Reply, Rejection> {
    logic::get_block_id(&context, &block_id_hex)
        .await
        .map_err(warp::reject::custom)?
        .ok_or(warp::reject::not_found())
}

#[derive(Deserialize)]
pub struct GetBlockNextIdQuery {
    count: Option<u32>,
}

pub async fn get_block_next_id(
    block_id_hex: String,
    query: GetBlockNextIdQuery,
    context: Context,
) -> Result<impl Reply, Rejection> {
    let count = query.count.unwrap_or(1);
    logic::get_block_next_id(&context, &block_id_hex, count as usize)
        .await
        .map_err(warp::reject::custom)?
        .ok_or(warp::reject::not_found())
}

pub async fn get_stake_distribution(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_stake_distribution(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_stake_distribution_at(
    epoch: u32,
    context: Context,
) -> Result<impl Reply, Rejection> {
    logic::get_stake_distribution_at(&context, epoch)
        .await
        .map_err(warp::reject::custom)?
        .map(|r| warp::reply::json(&r))
        .ok_or(warp::reject::not_found())
}

pub async fn get_settings(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_settings(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_shutdown(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_shutdown(&context)
        .await
        .map(|_| warp::reply())
        .map_err(warp::reject::custom)
}

pub async fn get_leaders(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_leader_ids(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn post_leaders(secret: NodeSecret, context: Context) -> Result<impl Reply, Rejection> {
    logic::post_leaders(&context, secret)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn delete_leaders(leader_id: u32, context: Context) -> Result<impl Reply, Rejection> {
    logic::delete_leaders(&context, leader_id.into())
        .await
        .map_err(warp::reject::custom)?
        .map(|()| warp::reply())
        .ok_or(warp::reject::not_found())
}

pub async fn get_leaders_logs(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_leaders_logs(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_stake_pools(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_stake_pools(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_stats(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_network_stats(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_rewards_info_epoch(epoch: u32, context: Context) -> Result<impl Reply, Rejection> {
    logic::get_rewards_info_epoch(&context, epoch)
        .await
        .map_err(warp::reject::custom)?
        .map(|r| warp::reply::json(&r))
        .ok_or(warp::reject::not_found())
}

pub async fn get_rewards_info_history(
    length: usize,
    context: Context,
) -> Result<impl Reply, Rejection> {
    logic::get_rewards_info_history(&context, length)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_utxo(
    fragment_id_hex: String,
    output_index: u8,
    context: Context,
) -> Result<impl Reply, Rejection> {
    logic::get_utxo(&context, &fragment_id_hex, output_index)
        .await
        .map_err(warp::reject::custom)?
        .map(|r| warp::reply::json(&r))
        .ok_or(warp::reject::not_found())
}

pub async fn get_stake_pool(
    pool_id_hex: String,
    context: Context,
) -> Result<impl Reply, Rejection> {
    logic::get_stake_pool(&context, &pool_id_hex)
        .await
        .map_err(warp::reject::custom)?
        .map(|r| warp::reply::json(&r))
        .ok_or(warp::reject::not_found())
}

pub async fn get_diagnostic(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_diagnostic(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_p2p_quarantined(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_network_p2p_quarantined(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_p2p_non_public(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_network_p2p_non_public(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_p2p_available(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_network_p2p_available(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_p2p_view(context: Context) -> Result<impl Reply, Rejection> {
    logic::get_network_p2p_view(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_network_p2p_view_topic(
    topic: String,
    context: Context,
) -> Result<impl Reply, Rejection> {
    logic::get_network_p2p_view_topic(&context, &topic)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}
