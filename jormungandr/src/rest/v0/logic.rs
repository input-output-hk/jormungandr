// This module contains framework-independent implementations of REST API
// methods. The convention is the following:
//
// - Everything returns Result<T, Error>
// - When the Ok type is Option<T> - None should be converted to 404
// - All errors should be processed on the framework  integration side. Usually
//   they are 400 or 500.
use std::net::SocketAddr;

use crate::{
    blockchain::StorageError,
    diagnostic::Diagnostic,
    intercom::{self, NetworkMsg, TransactionMsg},
    network::p2p::NodeProfile,
    rest::Context,
    secure::NodeSecret,
};
use chain_core::property::{
    Block as _, Deserialize, Fragment as fragment_property, FromStr, Serialize,
};
use chain_crypto::{
    bech32::Bech32, digest::Error as DigestError, hash::Error as HashError, Blake2b256, PublicKey,
    PublicKeyFromStrError,
};
use chain_impl_mockchain::{
    account::{AccountAlg, Identifier},
    block::Block as ChainBlock,
    fragment::{Fragment, FragmentId},
    key::Hash,
    leadership::{Leader, LeadershipConsensus},
    transaction::Transaction,
    value::{Value, ValueError},
};
use jormungandr_lib::{
    interfaces::{
        AccountState, EnclaveLeaderId, EpochRewardsInfo, FragmentLog, FragmentOrigin,
        LeadershipLog, NodeStats, NodeStatsDto, PeerStats, Rewards as StakePoolRewards,
        SettingsDto, StakeDistribution, StakeDistributionDto, StakePoolStats, TaxTypeSerde,
        TransactionOutput, VotePlanStatus,
    },
    time::SystemTime,
};

use std::sync::Arc;

use futures::{channel::mpsc::SendError, channel::mpsc::TrySendError, prelude::*};
use tracing::{span, Level};
use tracing_futures::Instrument;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ContextError(#[from] crate::rest::context::Error),
    #[error(transparent)]
    PublicKey(#[from] PublicKeyFromStrError),
    #[error(transparent)]
    IntercomError(#[from] intercom::Error),
    #[error(transparent)]
    Serialize(std::io::Error),
    #[error(transparent)]
    Deserialize(std::io::Error),
    #[error(transparent)]
    TxMsgSendError(#[from] TrySendError<TransactionMsg>),
    #[error(transparent)]
    MsgSendError(#[from] SendError),
    #[error("Block value calculation error")]
    Value(#[from] ValueError),
    #[error("Could not find block for tip")]
    TipBlockNotFound,
    #[error(transparent)]
    Hash(#[from] HashError),
    #[error(transparent)]
    Digest(#[from] DigestError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("Invalid topic")]
    InvalidTopic,
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
}

fn parse_account_id(id_hex: &str) -> Result<Identifier, Error> {
    PublicKey::<AccountAlg>::from_str(id_hex)
        .map(Into::into)
        .map_err(Into::into)
}

fn parse_block_hash(hex: &str) -> Result<Hash, Error> {
    Blake2b256::from_str(hex)
        .map_err(Into::into)
        .map(Into::into)
}

fn parse_fragment_id(id_hex: &str) -> Result<FragmentId, Error> {
    match FragmentId::from_str(id_hex) {
        Ok(id) => Ok(id),
        Err(e) => Err(e.into()),
    }
}

pub async fn get_account_state(
    context: &Context,
    account_id_hex: &str,
) -> Result<Option<AccountState>, Error> {
    Ok(context
        .blockchain_tip()?
        .get_ref()
        .await
        .ledger()
        .accounts()
        .get_state(&parse_account_id(account_id_hex)?)
        .ok()
        .map(Into::into))
}

pub async fn get_message_logs(context: &Context) -> Result<Vec<FragmentLog>, Error> {
    let span = span!(parent: context.span()?, Level::TRACE, "message_logs");
    async move {
        let (reply_handle, reply_future) = intercom::unary_reply();
        let mut mbox = context.try_full()?.transaction_task.clone();
        mbox.send(TransactionMsg::GetLogs(reply_handle))
            .await
            .map_err(|e| {
                tracing::debug!(reason = %e, "error getting message logs");
                Error::MsgSendError(e)
            })?;
        reply_future.await.map_err(Into::into)
    }
    .instrument(span)
    .await
}

pub async fn post_message(context: &Context, message: &[u8]) -> Result<String, Error> {
    let fragment = Fragment::deserialize(message).map_err(Error::Deserialize)?;
    let fragment_id = fragment.id().to_string();
    let msg = TransactionMsg::SendTransaction(FragmentOrigin::Rest, vec![fragment]);
    context.try_full()?.transaction_task.clone().try_send(msg)?;
    Ok(fragment_id)
}

pub async fn get_tip(context: &Context) -> Result<String, Error> {
    Ok(context.blockchain_tip()?.get_ref().await.hash().to_string())
}

pub async fn get_stats_counter(context: &Context) -> Result<NodeStatsDto, Error> {
    let stats = create_stats(&context).await?;
    Ok(NodeStatsDto {
        version: env!("SIMPLE_VERSION").to_string(),
        state: context.node_state().clone(),
        stats,
    })
}

async fn create_stats(context: &Context) -> Result<Option<NodeStats>, Error> {
    let (tip, blockchain, full_context) = match (
        context.blockchain_tip(),
        context.blockchain(),
        context.try_full(),
    ) {
        (Ok(tip), Ok(blockchain), Ok(full_context)) => (tip, blockchain, full_context),
        _ => return Ok(None),
    };

    let tip = tip.get_ref().await;

    let mut block_tx_count = 0u64;
    let mut block_input_sum = Value::zero();
    let mut block_fee_sum = Value::zero();

    let mut header_block = full_context.stats_counter.get_tip_block();

    // In case we do not have a cached block in the stats_counter we can retrieve it from the
    // storage, this should happen just once.
    if header_block.is_none() {
        let block: Option<ChainBlock> = blockchain.storage().get(tip.hash()).unwrap_or(None);

        // Update block if found
        if let Some(block) = block {
            full_context.stats_counter.set_tip_block(Arc::new(block));
        };

        header_block = full_context.stats_counter.get_tip_block();
    }

    header_block
        .as_ref()
        .as_ref()
        .ok_or(Error::TipBlockNotFound)?
        .contents
        .iter()
        .try_for_each::<_, Result<(), ValueError>>(|fragment| {
            fn totals<T>(t: &Transaction<T>) -> Result<(Value, Value), ValueError> {
                Ok((t.total_input()?, t.total_output()?))
            }

            let (total_input, total_output) = match &fragment {
                Fragment::Transaction(tx) => totals(tx),
                Fragment::OwnerStakeDelegation(tx) => totals(tx),
                Fragment::StakeDelegation(tx) => totals(tx),
                Fragment::PoolRegistration(tx) => totals(tx),
                Fragment::PoolRetirement(tx) => totals(tx),
                Fragment::PoolUpdate(tx) => totals(tx),
                Fragment::VotePlan(tx) => totals(tx),
                Fragment::VoteCast(tx) => totals(tx),
                Fragment::VoteTally(tx) => totals(tx),
                Fragment::EncryptedVoteTally(tx) => totals(tx),
                Fragment::Initial(_)
                | Fragment::OldUtxoDeclaration(_)
                | Fragment::UpdateProposal(_)
                | Fragment::UpdateVote(_) => return Ok(()),
            }?;
            block_tx_count += 1;
            block_input_sum = (block_input_sum + total_input)?;
            let fee = (total_input - total_output).unwrap_or_else(|_| Value::zero());
            block_fee_sum = (block_fee_sum + fee)?;
            Ok(())
        })?;

    let peer_available_cnt = get_network_p2p_view(&context).await?.len(); // FIXME
    let peer_quarantined_cnt = get_network_p2p_quarantined(&context).await?.len(); // FIXME
    let peer_total_cnt = peer_available_cnt + peer_quarantined_cnt;
    let tip_header = tip.header();
    let stats = &full_context.stats_counter;
    let node_stats = NodeStats {
        block_recv_cnt: stats.block_recv_cnt(),
        last_block_content_size: tip_header.block_content_size(),
        last_block_date: tip_header.block_date().to_string().into(),
        last_block_fees: block_fee_sum.0,
        last_block_hash: tip_header.hash().to_string().into(),
        last_block_height: tip_header.chain_length().to_string().into(),
        last_block_sum: block_input_sum.0,
        last_block_time: SystemTime::from(tip.time()).into(),
        last_block_tx: block_tx_count,
        last_received_block_time: stats.slot_start_time().map(SystemTime::from),
        peer_available_cnt,
        peer_connected_cnt: stats.peer_connected_cnt(),
        peer_quarantined_cnt,
        peer_total_cnt,
        peer_unreachable_cnt: 0, // FIXME
        tx_recv_cnt: stats.tx_recv_cnt(),
        uptime: stats.uptime_sec().into(),
    };
    Ok(Some(node_stats))
}

pub async fn get_block_id(context: &Context, block_id_hex: &str) -> Result<Option<Vec<u8>>, Error> {
    context
        .blockchain()?
        .storage()
        .get(parse_block_hash(&block_id_hex)?)?
        .map(|b| b.serialize_as_vec().map_err(Error::Serialize))
        .transpose()
}

pub async fn get_block_next_id(
    context: &Context,
    block_id_hex: &str,
    count: usize,
) -> Result<Option<Vec<u8>>, Error> {
    let blockchain = context.blockchain()?;
    let block_id = parse_block_hash(&block_id_hex)?;
    let tip = context.blockchain_tip()?.get_ref().await;
    let maybe_stream = blockchain
        .storage()
        .stream_from_to(block_id, tip.hash())
        .map(Some)
        .or_else(|e| match e {
            StorageError::CannotIterate | StorageError::BlockNotFound => Ok(None),
            e => Err(Error::Storage(e)),
        })?;

    if let Some(stream) = maybe_stream {
        Some(
            stream
                .map_err(Into::into)
                .take(count)
                .try_fold(Vec::new(), |mut bytes, block| async move {
                    bytes.extend_from_slice(block.id().as_ref());
                    Ok(bytes)
                })
                .await,
        )
        .transpose()
    } else {
        Ok(None)
    }
}

pub async fn get_stake_distribution(
    context: &Context,
) -> Result<Option<StakeDistributionDto>, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    let leadership = blockchain_tip.epoch_leadership_schedule();
    if let LeadershipConsensus::GenesisPraos(gp) = leadership.consensus() {
        let last_epoch = blockchain_tip.block_date().epoch;
        let distribution = gp.distribution();
        let stake = StakeDistribution {
            dangling: distribution.dangling.into(),
            unassigned: distribution.unassigned.into(),
            pools: distribution
                .to_pools
                .iter()
                .map(|(key, value)| (key.clone().into(), value.stake.total.clone().into()))
                .collect(),
        };
        Ok(Some(StakeDistributionDto {
            epoch: last_epoch,
            stake,
        }))
    } else {
        Ok(None)
    }
}

pub async fn get_stake_distribution_at(
    context: &Context,
    epoch: u32,
) -> Result<Option<StakeDistributionDto>, Error> {
    let mut tip_ref = context.blockchain_tip()?.get_ref().await;

    if epoch > tip_ref.block_date().epoch {
        return Ok(None);
    }

    loop {
        if tip_ref.block_date().epoch == epoch {
            break;
        }
        match tip_ref.last_ref_previous_epoch() {
            Some(previous_epoch) => {
                if epoch > previous_epoch.block_date().epoch {
                    return Ok(None);
                }
                tip_ref = Arc::clone(previous_epoch);
            }
            _ => return Ok(None),
        }
    }

    Ok(tip_ref
        .epoch_leadership_schedule()
        .stake_distribution()
        .map(|distribution| {
            let stake = StakeDistribution {
                dangling: distribution.dangling.into(),
                unassigned: distribution.unassigned.into(),
                pools: distribution
                    .to_pools
                    .iter()
                    .map(|(key, value)| (key.clone().into(), value.stake.total.clone().into()))
                    .collect(),
            };

            StakeDistributionDto { epoch, stake }
        }))
}

pub async fn get_settings(context: &Context) -> Result<SettingsDto, Error> {
    let full_context = context.try_full()?;
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
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
    Ok(SettingsDto {
        block0_hash: static_params.block0_initial_hash.to_string(),
        block0_time: SystemTime::from_secs_since_epoch(static_params.block0_start_time.0),
        curr_slot_start_time: full_context
            .stats_counter
            .slot_start_time()
            .map(SystemTime::from),
        consensus_version: consensus_version.to_string(),
        fees,
        block_content_max_size,
        epoch_stability_depth,
        slot_duration: blockchain_tip.time_frame().slot_duration(),
        slots_per_epoch,
        treasury_tax: current_params.treasury_tax,
        reward_params: current_params.reward_params.clone(),
    })
}

pub async fn shutdown(context: &mut Context) -> Result<(), Error> {
    context.stop_bootstrap();
    context.server_stopper()?.stop();
    Ok(())
}

pub async fn get_leader_ids(context: &Context) -> Result<Vec<EnclaveLeaderId>, Error> {
    Ok(context.try_full()?.enclave.get_leader_ids().await)
}

pub async fn post_leaders(context: &Context, secret: NodeSecret) -> Result<EnclaveLeaderId, Error> {
    let leader = Leader {
        bft_leader: secret.bft(),
        genesis_leader: secret.genesis(),
    };
    let leader_id = context.try_full()?.enclave.add_leader(leader).await;
    Ok(leader_id)
}

pub async fn delete_leaders(
    context: &Context,
    leader_id: EnclaveLeaderId,
) -> Result<Option<()>, Error> {
    let removed = context.try_full()?.enclave.remove_leader(leader_id).await;

    if removed {
        Ok(Some(()))
    } else {
        Ok(None)
    }
}

pub async fn get_leaders_logs(context: &Context) -> Result<Vec<LeadershipLog>, Error> {
    Ok(context.try_full()?.leadership_logs.logs().await)
}

pub async fn get_stake_pools(context: &Context) -> Result<Vec<String>, Error> {
    Ok(context
        .blockchain_tip()?
        .get_ref()
        .await
        .ledger()
        .delegation()
        .stake_pool_ids()
        .map(|id| id.to_string())
        .collect())
}

pub async fn get_network_stats(context: &Context) -> Result<Vec<PeerStats>, Error> {
    let full_context = context.try_full()?;

    let span = span!(parent: context.span()?, Level::TRACE, "request", request = "network_stats");
    async move {
        let (reply_handle, reply_future) = intercom::unary_reply();
        let mut mbox = full_context.network_task.clone();
        mbox.send(NetworkMsg::PeerInfo(reply_handle))
            .await
            .map_err(|e| {
                tracing::debug!(reason = %e, "error getting network stats");
                Error::MsgSendError(e)
            })?;
        let peer_stats = reply_future.await?;
        Ok(peer_stats
            .into_iter()
            .map(|info| PeerStats {
                addr: info.addr,
                established_at: SystemTime::from(info.stats.connection_established()),
                last_block_received: info.stats.last_block_received().map(SystemTime::from),
                last_fragment_received: info.stats.last_fragment_received().map(SystemTime::from),
                last_gossip_received: info.stats.last_gossip_received().map(SystemTime::from),
            })
            .collect())
    }
    .instrument(span)
    .await
}

pub async fn get_rewards_info_epoch(
    context: &Context,
    epoch: u32,
) -> Result<Option<EpochRewardsInfo>, Error> {
    let mut tip_ref = context.blockchain_tip()?.get_ref().await;

    if epoch > tip_ref.block_date().epoch {
        return Ok(None);
    }

    loop {
        if tip_ref.block_date().epoch == epoch {
            break;
        }
        match tip_ref.last_ref_previous_epoch() {
            Some(previous_epoch) => {
                if epoch > previous_epoch.block_date().epoch {
                    return Ok(None);
                }
                tip_ref = Arc::clone(previous_epoch);
            }
            _ => return Ok(None),
        }
    }

    if let Some(epoch_rewards_info) = tip_ref.epoch_rewards_info() {
        Ok(Some(EpochRewardsInfo::from(
            tip_ref.block_date().epoch,
            epoch_rewards_info.as_ref(),
        )))
    } else {
        Ok(None)
    }
}

pub async fn get_rewards_info_history(
    context: &Context,
    length: usize,
) -> Result<Vec<EpochRewardsInfo>, Error> {
    let mut tip_ref = context.blockchain_tip()?.get_ref().await;

    let mut vec = Vec::new();
    while let Some(epoch_rewards_info) = tip_ref.epoch_rewards_info() {
        vec.push(EpochRewardsInfo::from(
            tip_ref.block_date().epoch,
            epoch_rewards_info.as_ref(),
        ));

        if let Some(previous_epoch) = tip_ref.last_ref_previous_epoch() {
            tip_ref = Arc::clone(previous_epoch);
        } else {
            break;
        }

        if vec.len() >= length {
            break;
        }
    }

    Ok(vec)
}

pub async fn get_utxo(
    context: &Context,
    fragment_id_hex: &str,
    output_index: u8,
) -> Result<Option<TransactionOutput>, Error> {
    let fragment_id = parse_fragment_id(fragment_id_hex)?;
    Ok(context
        .blockchain_tip()?
        .get_ref()
        .await
        .ledger()
        .utxo_out(fragment_id, output_index)
        .cloned()
        .map(Into::into))
}

pub async fn get_stake_pool(
    context: &Context,
    pool_id_hex: &str,
) -> Result<Option<StakePoolStats>, Error> {
    let pool_id = pool_id_hex.parse()?;
    let ledger = context.blockchain_tip()?.get_ref().await.ledger();
    Ok(ledger.delegation().lookup(&pool_id).map(|pool| {
        let total_stake: u64 = ledger
            .get_stake_distribution()
            .to_pools
            .get(&pool_id)
            .map(|pool| pool.stake.total.into())
            .unwrap_or(0);
        StakePoolStats {
            kes_public_key: pool.registration.keys.kes_public_key.to_bech32_str(),
            vrf_public_key: pool.registration.keys.vrf_public_key.to_bech32_str(),
            total_stake,
            rewards: StakePoolRewards {
                epoch: pool.last_rewards.epoch,
                value_taxed: pool.last_rewards.value_taxed,
                value_for_stakers: pool.last_rewards.value_for_stakers,
            },
            tax: TaxTypeSerde(pool.registration.rewards),
        }
    }))
}

pub async fn get_diagnostic(context: &Context) -> Result<Diagnostic, Error> {
    let diagnostic_data = context.get_diagnostic_data()?;
    Ok(diagnostic_data.clone())
}

pub async fn get_network_p2p_quarantined(context: &Context) -> Result<Vec<NodeProfile>, Error> {
    Ok(context
        .try_full()?
        .network_state
        .topology()
        .list_quarantined()
        .await)
}

pub async fn get_network_p2p_non_public(context: &Context) -> Result<Vec<NodeProfile>, Error> {
    Ok(context
        .try_full()?
        .network_state
        .topology()
        .list_non_public()
        .await)
}

pub async fn get_network_p2p_available(context: &Context) -> Result<Vec<NodeProfile>, Error> {
    Ok(context
        .try_full()?
        .network_state
        .topology()
        .list_available()
        .await)
}

pub async fn get_network_p2p_view(context: &Context) -> Result<Vec<SocketAddr>, Error> {
    Ok(context
        .try_full()?
        .network_state
        .topology()
        .view(poldercast::layer::Selection::Any)
        .await
        .peers
        .into_iter()
        .map(|peer| peer.address())
        .collect())
}

pub async fn get_network_p2p_view_topic(
    context: &Context,
    topic: &str,
) -> Result<Vec<SocketAddr>, Error> {
    fn parse_topic(s: &str) -> Result<poldercast::layer::Selection, Error> {
        use crate::network::p2p::topic;
        use poldercast::layer::Selection;
        match s {
            "blocks" => Ok(Selection::Topic {
                topic: topic::BLOCKS,
            }),
            "fragments" => Ok(Selection::Topic {
                topic: topic::MESSAGES,
            }),
            "" => Ok(Selection::Any),
            _ => Err(Error::InvalidTopic),
        }
    }

    let topic = parse_topic(topic)?;
    let ctx = context.try_full()?;
    let view = ctx.network_state.topology().view(topic).await;
    Ok(view.peers.into_iter().map(|peer| peer.address()).collect())
}

pub async fn get_committees(context: &Context) -> Result<Vec<String>, Error> {
    Ok(context
        .blockchain_tip()?
        .get_ref()
        .await
        .epoch_ledger_parameters()
        .committees
        .to_vec()
        .iter()
        .map(|cid| cid.to_string())
        .collect())
}

pub async fn get_active_vote_plans(context: &Context) -> Result<Vec<VotePlanStatus>, Error> {
    let vp = context
        .blockchain_tip()?
        .get_ref()
        .await
        .active_vote_plans()
        .into_iter()
        .map(VotePlanStatus::from)
        .collect();
    Ok(vp)
}
