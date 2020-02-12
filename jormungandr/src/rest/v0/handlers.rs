use jormungandr_lib::interfaces::{
    AccountState, Address, EnclaveLeaderId, FragmentOrigin, Rewards as StakePoolRewards,
    StakePoolStats, TaxTypeSerde,
};
use jormungandr_lib::time::SystemTime;

use actix_web::error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound};
use actix_web::web::{Bytes, BytesMut, Data, Json, Path, Query};
use actix_web::{Error, HttpResponse, Responder};
use chain_core::property::{Block, Deserialize, Serialize as _};
use chain_crypto::{bech32::Bech32, Blake2b256, PublicKey};
use chain_impl_mockchain::account::{AccountAlg, Identifier};
use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use chain_impl_mockchain::key::Hash;
use chain_impl_mockchain::leadership::{Leader, LeadershipConsensus};
use chain_impl_mockchain::stake::StakeDistribution;
use chain_impl_mockchain::transaction::Transaction;
use chain_impl_mockchain::value::{Value, ValueError};
use chain_storage::error::Error as StorageError;

use crate::blockchain::Ref;
use crate::intercom::{self, NetworkMsg, TransactionMsg};
use crate::secure::NodeSecret;
use futures::Stream;
use futures03::compat::Future01CompatExt;
use jormungandr_lib::interfaces::NodeState;
use std::str::FromStr;
use std::sync::Arc;

pub use crate::rest::{Context, FullContext};

async fn chain_tip(context: &Data<Context>) -> Result<Arc<Ref>, Error> {
    chain_tip_from_full(&*context.try_full().await?).await
}

async fn chain_tip_from_full(context: &FullContext) -> Result<Arc<Ref>, Error> {
    context.blockchain_tip.get_ref().compat().await
}

fn parse_account_id(id_hex: &str) -> Result<Identifier, Error> {
    PublicKey::<AccountAlg>::from_str(id_hex)
        .map(Into::into)
        .map_err(ErrorBadRequest)
}

fn parse_fragment_id(id_hex: &str) -> Result<FragmentId, Error> {
    FragmentId::from_str(id_hex).map_err(ErrorBadRequest)
}

fn parse_block_hash(hex: &str) -> Result<Hash, Error> {
    Blake2b256::from_str(hex)
        .map_err(ErrorBadRequest)
        .map(Into::into)
}

pub async fn get_account_state(
    context: Data<Context>,
    account_id_hex: Path<String>,
) -> Result<impl Responder, Error> {
    let account_id = parse_account_id(&account_id_hex)?;
    let chain_tip = chain_tip(&context).await?;
    let ledger = chain_tip.ledger();
    let state = ledger
        .accounts()
        .get_state(&account_id)
        .map_err(ErrorNotFound)?;
    Ok(Json(AccountState::from(state)))
}

pub async fn get_message_logs(context: Data<Context>) -> Result<impl Responder, Error> {
    context
        .try_full()
        .await?
        .logs
        .logs()
        .await
        .map_err(|_| ErrorInternalServerError("Failed to get logs"))
        .map(Json)
}

pub async fn post_message(context: Data<Context>, message: Bytes) -> Result<impl Responder, Error> {
    let fragment = Fragment::deserialize(&*message).map_err(ErrorBadRequest)?;
    let msg = TransactionMsg::SendTransaction(FragmentOrigin::Rest, vec![fragment]);
    context
        .try_full()
        .await?
        .transaction_task
        .clone()
        .try_send(msg)
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn get_tip(context: Data<Context>) -> Result<impl Responder, Error> {
    chain_tip(&context).await.map(|tip| tip.hash().to_string())
}

#[derive(Serialize)]
struct NodeStatsDto {
    version: &'static str,
    state: NodeState,
    #[serde(flatten)]
    stats: Option<serde_json::Value>,
}

pub async fn get_stats_counter(context: Data<Context>) -> Result<impl Responder, Error> {
    let stats = match context.try_full().await {
        Ok(full_context) => Some(create_stats(&*full_context).await?),
        Err(_) => None,
    };
    Ok(Json(NodeStatsDto {
        version: env!("SIMPLE_VERSION"),
        state: context.node_state().await,
        stats,
    }))
}

async fn create_stats(context: &FullContext) -> Result<serde_json::Value, Error> {
    let tip = chain_tip_from_full(context).await?;
    let mut block_tx_count = 0u64;
    let mut block_input_sum = Value::zero();
    let mut block_fee_sum = Value::zero();
    context
        .blockchain
        .storage()
        .get(tip.hash())
        .compat()
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or(ErrorInternalServerError("Could not find block for tip"))?
        .contents
        .iter()
        .map(|fragment| {
            fn totals<T>(t: &Transaction<T>) -> Result<(Value, Value), ValueError> {
                Ok((t.total_input()?, t.total_output()?))
            }

            let (total_input, total_output) = match fragment {
                Fragment::Transaction(tx) => totals(tx),
                Fragment::OwnerStakeDelegation(tx) => totals(tx),
                Fragment::StakeDelegation(tx) => totals(tx),
                Fragment::PoolRegistration(tx) => totals(tx),
                Fragment::PoolRetirement(tx) => totals(tx),
                Fragment::PoolUpdate(tx) => totals(tx),
                Fragment::Initial(_)
                | Fragment::OldUtxoDeclaration(_)
                | Fragment::UpdateProposal(_)
                | Fragment::UpdateVote(_) => return Ok(()),
            }?;
            block_tx_count += 1;
            block_input_sum = (block_input_sum + total_input)?;
            let fee = (total_input - total_output).unwrap_or(Value::zero());
            block_fee_sum = (block_fee_sum + fee)?;
            Ok(())
        })
        .collect::<Result<(), ValueError>>()
        .map_err(|e| ErrorInternalServerError(format!("Block value calculation error: {}", e)))?;
    let nodes_count = &context.p2p.nodes_count::<Error>().compat().await?;
    let tip_header = tip.header();
    let stats = &context.stats_counter;
    let node_id = &context.p2p.node_id().to_string();
    Ok(json!({
        "txRecvCnt": stats.tx_recv_cnt(),
        "blockRecvCnt": stats.block_recv_cnt(),
        "lastReceivedBlockTime": stats.slot_start_time().map(SystemTime::from),
        "uptime": stats.uptime_sec(),
        "lastBlockHash": tip_header.hash().to_string(),
        "lastBlockHeight": tip_header.chain_length().to_string(),
        "lastBlockDate": tip_header.block_date().to_string(),
        "lastBlockTime": SystemTime::from(tip.time()),
        "lastBlockTx": block_tx_count,
        "lastBlockContentSize": tip_header.block_content_size(),
        "lastBlockSum": block_input_sum.0,
        "lastBlockFees": block_fee_sum.0,
        "peerTotalCnt": nodes_count.all_count,
        "peerAvailableCnt": nodes_count.available_count,
        "peerUnreachableCnt": nodes_count.not_reachable_count,
        "peerQuarantinedCnt": nodes_count.quarantined_count,
        "nodeId": node_id,
    }))
}

pub async fn get_block_id(
    context: Data<Context>,
    block_id_hex: Path<String>,
) -> Result<impl Responder, Error> {
    context
        .try_full()
        .await?
        .blockchain
        .storage()
        .get(parse_block_hash(&block_id_hex)?)
        .compat()
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or(ErrorNotFound("Block not found"))?
        .serialize_as_vec()
        .map_err(ErrorInternalServerError)
        .map(Bytes::from)
}

pub async fn get_block_next_id(
    context: Data<Context>,
    block_id_hex: Path<String>,
    query_params: Query<QueryParams>,
) -> Result<impl Responder, Error> {
    let full_context = context.try_full().await?;
    let block_id = parse_block_hash(&block_id_hex)?;
    let tip = chain_tip_from_full(&full_context).await?;
    full_context
        .blockchain
        .storage()
        .stream_from_to(block_id, tip.hash())
        .compat()
        .await
        .map_err(|e| match e {
            StorageError::CannotIterate => ErrorNotFound("Block is not in chain of the tip"),
            StorageError::BlockNotFound => ErrorNotFound(e),
            _ => ErrorInternalServerError(e),
        })?
        .map_err(ErrorInternalServerError)
        .take(query_params.get_count())
        .fold(BytesMut::new(), |mut bytes, block| {
            bytes.extend_from_slice(block.id().as_ref());
            Result::<BytesMut, Error>::Ok(bytes)
        })
        .compat()
        .await
}

const MAX_COUNT: u64 = 100;

#[derive(Deserialize)]
pub struct QueryParams {
    count: Option<u64>,
}

impl QueryParams {
    pub fn get_count(&self) -> u64 {
        self.count.unwrap_or(1).min(MAX_COUNT)
    }
}

pub async fn get_stake_distribution(context: Data<Context>) -> Result<impl Responder, Error> {
    let blockchain_tip = chain_tip(&context).await?;
    let leadership = blockchain_tip.epoch_leadership_schedule();
    let last_epoch = blockchain_tip.block_date().epoch;
    let stake = if let LeadershipConsensus::GenesisPraos(gp) = leadership.consensus() {
        Some(create_stake(gp.distribution()))
    } else {
        None
    };
    Ok(Json(json!({
        "epoch": last_epoch,
        "stake": stake,
    })))
}

fn create_stake(stake: &StakeDistribution) -> serde_json::Value {
    let unassigned: u64 = stake.unassigned.into();
    let dangling: u64 = stake.dangling.into();
    let pools: Vec<(String, u64)> = stake
        .to_pools
        .iter()
        .map(|(h, p)| (format!("{}", h), p.stake.total.into()))
        .collect();
    json!({
        "unassigned": unassigned,
        "dangling": dangling,
        "pools": pools,
    })
}

pub async fn get_settings(context: Data<Context>) -> Result<impl Responder, Error> {
    let full_context = context.try_full().await?;
    let blockchain_tip = chain_tip_from_full(&full_context).await?;
    let ledger = blockchain_tip.ledger();
    let static_params = ledger.get_static_parameters();
    let consensus_version = ledger.consensus_version();
    let current_params = blockchain_tip.epoch_ledger_parameters();
    let fees = current_params.fees;
    let block_content_max_size = current_params.block_content_max_size;
    let epoch_stability_depth = current_params.epoch_stability_depth;
    let slots_per_epoch = blockchain_tip
        .epoch_leadership_schedule()
        .era()
        .slots_per_epoch();
    let settings = jormungandr_lib::interfaces::SettingsDto {
        block0_hash: static_params.block0_initial_hash.to_string(),
        block0_time: SystemTime::from_secs_since_epoch(static_params.block0_start_time.0),
        curr_slot_start_time: full_context
            .stats_counter
            .slot_start_time()
            .map(SystemTime::from),
        consensus_version: consensus_version.to_string(),
        fees: fees,
        block_content_max_size: block_content_max_size,
        epoch_stability_depth: epoch_stability_depth,
        slot_duration: blockchain_tip.time_frame().slot_duration(),
        slots_per_epoch,
        treasury_tax: current_params.treasury_tax,
        reward_params: current_params.reward_params.clone(),
    };
    Ok(Json(json!(settings)))
}

pub async fn get_shutdown(context: Data<Context>) -> Result<impl Responder, Error> {
    // Verify that node has fully started and is able to process shutdown
    context.try_full().await?;
    // Server finishes ongoing tasks before stopping, so user will get response to this request
    // Node should be shutdown automatically when server stopping is finished
    context.server_stopper().await?.stop();
    Ok(HttpResponse::Ok().finish())
}

pub async fn get_leaders(context: Data<Context>) -> Result<impl Responder, Error> {
    Ok(Json(json! {
        context.try_full().await?.enclave.get_leaderids().await
    }))
}

pub async fn post_leaders(
    secret: Json<NodeSecret>,
    context: Data<Context>,
) -> Result<impl Responder, Error> {
    let leader = Leader {
        bft_leader: secret.bft(),
        genesis_leader: secret.genesis(),
    };
    let leader_id = context.try_full().await?.enclave.add_leader(leader).await;
    Ok(Json(leader_id))
}

pub async fn delete_leaders(
    context: Data<Context>,
    leader_id: Path<EnclaveLeaderId>,
) -> Result<impl Responder, Error> {
    match context
        .try_full()
        .await?
        .enclave
        .remove_leader(*leader_id)
        .await
    {
        true => Ok(HttpResponse::Ok().finish()),
        false => Err(ErrorNotFound("Leader with given ID not found")),
    }
}

pub async fn get_leaders_logs(context: Data<Context>) -> Result<impl Responder, Error> {
    Ok(Json(context.try_full().await?.leadership_logs.logs().await))
}

pub async fn get_stake_pools(context: Data<Context>) -> Result<impl Responder, Error> {
    let stake_pool_ids = chain_tip(&context)
        .await?
        .ledger()
        .delegation()
        .stake_pool_ids()
        .map(|id| id.to_string())
        .collect::<Vec<_>>();
    Ok(Json(stake_pool_ids))
}

pub async fn get_network_stats(context: Data<Context>) -> Result<impl Responder, Error> {
    let full_context = context.try_full().await?;
    let peer_stats = intercom::unary_future(
        full_context.network_task.clone(),
        context.logger().await?,
        |reply_handle| NetworkMsg::PeerInfo(reply_handle),
    )
    .compat()
    .await
    .map_err(|e: intercom::Error| ErrorInternalServerError(e))?;
    let network_stats = peer_stats
        .into_iter()
        .map(|info| {
            json! ({
                "nodeId": info.id.to_string(),
                "addr": info.addr,
                "establishedAt": SystemTime::from(info.stats.connection_established()),
                "lastBlockReceived": info.stats.last_block_received().map(SystemTime::from),
                "lastFragmentReceived": info.stats.last_fragment_received().map(SystemTime::from),
                "lastGossipReceived": info.stats.last_gossip_received().map(SystemTime::from),
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(network_stats))
}

pub async fn get_utxo(
    context: Data<Context>,
    path_params: Path<(String, u8)>,
) -> Result<impl Responder, Error> {
    let (fragment_id_hex, output_index) = path_params.into_inner();
    let fragment_id = parse_fragment_id(&fragment_id_hex)?;
    let tip_reference = chain_tip(&context).await?;
    let ledger = tip_reference.ledger();
    let output = ledger.utxo_out(fragment_id, output_index).ok_or_else(|| {
        ErrorNotFound(format!(
            "no UTxO found for address '{}' on index {}",
            fragment_id_hex, output_index
        ))
    })?;
    Ok(Json(json!({
        "address": Address::from(output.address.clone()),
        "value": output.value.0,
    })))
}

pub async fn get_stake_pool(
    context: Data<Context>,
    pool_id_hex: Path<String>,
) -> Result<impl Responder, Error> {
    let pool_id = pool_id_hex.parse().map_err(ErrorBadRequest)?;
    let chain_tip = chain_tip(&context).await?;
    let ledger = chain_tip.ledger();
    let pool = ledger
        .delegation()
        .lookup(&pool_id)
        .ok_or_else(|| ErrorNotFound(format!("Stake pool '{}' not found", pool_id_hex)))?;
    let total_stake: u64 = ledger.get_stake_distribution().get_total_stake().into();
    Ok(Json(json!(StakePoolStats {
        kes_public_key: pool
            .registration
            .keys
            .kes_public_key
            .to_bech32_str()
            .to_owned(),
        vrf_public_key: pool
            .registration
            .keys
            .vrf_public_key
            .to_bech32_str()
            .to_owned(),
        total_stake: total_stake,
        rewards: StakePoolRewards {
            epoch: pool.last_rewards.epoch,
            value_taxed: pool.last_rewards.value_taxed,
            value_for_stakers: pool.last_rewards.value_for_stakers,
        },
        tax: TaxTypeSerde(pool.registration.rewards),
    })))
}

pub async fn get_diagnostic(context: Data<Context>) -> Result<impl Responder, Error> {
    let full_context = context.try_full().await?;
    serde_json::to_string(&full_context.diagnostic).map_err(ErrorInternalServerError)
}

pub async fn get_network_p2p_quarantined(context: Data<Context>) -> Result<impl Responder, Error> {
    let ctx = context.try_full().await?;
    let list = ctx.p2p.list_quarantined::<Error>().compat().await?;
    Ok(Json(json!(list)))
}

pub async fn get_network_p2p_non_public(context: Data<Context>) -> Result<impl Responder, Error> {
    let ctx = context.try_full().await?;
    let list = ctx.p2p.list_non_public::<Error>().compat().await?;
    Ok(Json(json!(list)))
}

pub async fn get_network_p2p_available(context: Data<Context>) -> Result<impl Responder, Error> {
    let ctx = context.try_full().await?;
    let list = ctx.p2p.list_available::<Error>().compat().await?;
    Ok(Json(json!(list)))
}

pub async fn get_network_p2p_view(context: Data<Context>) -> Result<impl Responder, Error> {
    let ctx = context.try_full().await?;
    let view = ctx
        .p2p
        .view::<Error>(poldercast::Selection::Any)
        .compat()
        .await?;
    let node_infos: Vec<poldercast::NodeInfo> = view.peers.into_iter().map(Into::into).collect();
    Ok(Json(json!(node_infos)))
}

pub async fn get_network_p2p_view_topic(
    context: Data<Context>,
    topic: Path<String>,
) -> Result<impl Responder, Error> {
    fn parse_topic(s: &str) -> Result<poldercast::Selection, Error> {
        use crate::network::p2p::topic;
        use poldercast::Selection;
        match s {
            "blocks" => Ok(Selection::Topic {
                topic: topic::BLOCKS,
            }),
            "fragments" => Ok(Selection::Topic {
                topic: topic::MESSAGES,
            }),
            "" => Ok(Selection::Any),
            _ => Err(ErrorBadRequest("invalid topic")),
        }
    }

    let topic = parse_topic(&topic.into_inner())?;
    let ctx = context.try_full().await?;
    let view = ctx.p2p.view::<Error>(topic).compat().await?;
    let node_infos: Vec<poldercast::NodeInfo> = view.peers.into_iter().map(Into::into).collect();
    Ok(Json(json!(node_infos)))
}
