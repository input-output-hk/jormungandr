use crate::rest::{v1::logic, ContextLock};
use jormungandr_lib::interfaces::{FragmentsBatch, VotePlanId};
use warp::{reject::Reject, Rejection, Reply};

impl Reject for logic::Error {}

pub async fn post_fragments(
    fragments: FragmentsBatch,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::post_fragments(&context, fragments)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

#[derive(Deserialize)]
pub struct GetMessageStatusesQuery {
    fragment_ids: String,
}

pub async fn get_fragment_statuses(
    query: GetMessageStatusesQuery,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    let fragment_ids = query.fragment_ids.split(',');
    logic::get_fragment_statuses(&context, fragment_ids)
        .await
        .map_err(warp::reject::custom)
        .map(|r| warp::reply::json(&r))
}

pub async fn get_fragment_logs(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_fragment_logs(&context)
        .await
        .map_err(warp::reject::custom)
        .map(|r| warp::reply::json(&r))
}

pub async fn get_account_votes_with_plan(
    vote_plan_id: VotePlanId,
    account_id_hex: String,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_account_votes_with_plan(&context, vote_plan_id, account_id_hex)
        .await
        .map_err(warp::reject::custom)?
        .ok_or_else(warp::reject::not_found)
        .map(|r| warp::reply::json(&r))
}

pub async fn get_account_votes(
    account_id_hex: String,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_account_votes(&context, account_id_hex)
        .await
        .map_err(warp::reject::custom)?
        .ok_or_else(warp::reject::not_found)
        .map(|r| warp::reply::json(&r))
}

pub async fn get_accounts_votes_all(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_accounts_votes_all(&context)
        .await
        .map_err(warp::reject::custom)
        .map(|r| warp::reply::json(&r))
}
