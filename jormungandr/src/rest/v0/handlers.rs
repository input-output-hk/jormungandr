use crate::{
    rest::{v0::logic, ContextLock},
    secure::NodeSecret,
};
use serde::Serialize;
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
        .ok_or(warp::reject::not_found())
}

pub async fn get_message_logs(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_message_logs(&context)
        .await
        .map_err(warp::reject::custom)
        .map(|r| warp::reply::json(&r))
}

pub async fn post_message(
    message: bytes::Bytes,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::post_message(&context, &message)
        .await
        .map(|()| warp::reply())
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
        .ok_or(warp::reject::not_found())
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
        .ok_or(warp::reject::not_found())
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
        .ok_or(warp::reject::not_found())
}

pub async fn get_settings(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_settings(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn get_shutdown(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_shutdown(&context)
        .await
        .map(|_| warp::reply())
        .map_err(warp::reject::custom)
}

pub async fn get_leaders(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::get_leader_ids(&context)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn post_leaders(
    secret: NodeSecret,
    context: ContextLock,
) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::post_leaders(&context, secret)
        .await
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}

pub async fn delete_leaders(leader_id: u32, context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    logic::delete_leaders(&context, leader_id.into())
        .await
        .map_err(warp::reject::custom)?
        .map(|()| warp::reply())
        .ok_or(warp::reject::not_found())
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
        .ok_or(warp::reject::not_found())
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
        .ok_or(warp::reject::not_found())
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
        .ok_or(warp::reject::not_found())
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

#[derive(Serialize)]
#[serde(remote="chain_impl_mockchain::block::BlockDate")]
struct BlockDate {
    pub epoch: u32,
    pub slot_id: u32
}

#[derive(Serialize)]
#[serde(remote="chain_impl_mockchain::certificate::ExternalProposalId")]
struct ExternalProposalId {
    #[serde(getter = "chain_impl_mockchain::certificate::ExternalProposalId::to_string")]
    id: String
}

#[derive(Serialize)]
#[serde(remote="chain_impl_mockchain::certificate::VoteOptions")]
struct VoteOptions {
    #[serde(getter = "chain_impl_mockchain::certificate::VoteOptions::as_byte")]
    num_choices: u8
}

#[derive(Serialize)]
// #[serde(remote="chain_impl_mockchain::certificate::Proposal")]
pub struct Proposal {
    // #[serde(with="ExternalProposalId", getter="chain_impl_mockchain::certificate::Proposal::external_id")]
    pub external_id: String,
    // #[serde(with="VoteOptions", getter="chain_impl_mockchain::certificate::Proposal::options")]
    pub options: u8,
}

impl Proposal {
    fn new(p: &chain_impl_mockchain::certificate::Proposal) -> Self {
        Self {
            external_id: p.external_id().to_string(),
            options: p.options().as_byte(),
        }
    }
}

#[derive(Serialize)]
struct VotePlan {
    /// the vote start validity
    #[serde(with="BlockDate")]
    pub vote_start: chain_impl_mockchain::block::BlockDate,
    /// the duration within which it is possible to vote for one of the proposals
    /// of this voting plan.
    #[serde(with="BlockDate")]
    pub vote_end: chain_impl_mockchain::block::BlockDate,
    /// the committee duration is the time allocated to the committee to open
    /// the ballots and publish the results on chain
    #[serde(with="BlockDate")]
    pub committee_end: chain_impl_mockchain::block::BlockDate,
    /// the proposals to vote for
    pub proposals: Vec<Proposal>,
}

impl VotePlan {
    pub fn from_vote_plan(plan: &chain_impl_mockchain::certificate::VotePlan) -> Self {
        VotePlan {
            vote_start: plan.vote_start(),
            vote_end: plan.vote_end(),
            committee_end: plan.committee_end(),
            proposals: plan.proposals().iter().map(Proposal::new).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct VotePlans {
    plans: Vec<VotePlan>
}

impl VotePlans {
    fn new(plans: Vec<VotePlan>) -> Self {
        Self {
            plans
        }
    }
}

pub async fn get_active_vote_plans(context: ContextLock) -> Result<impl Reply, Rejection> {
    let context = context.read().await;
    let vote_plans = logic::get_active_vote_plans(&context).await;
    vote_plans
        .map(|v| v.iter().map(|p| VotePlan::from_vote_plan(p)).collect::<Vec<VotePlan>>())
        .map(VotePlans::new)
        .map(|r| warp::reply::json(&r))
        .map_err(warp::reject::custom)
}
