use actix_web::error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound};
use actix_web::web::{Bytes, Data, Json, Path, Query};
use actix_web::{Error, HttpResponse, Responder};
use jormungandr_lib::interfaces::EnclaveLeaderId;

use crate::rest::v0::logic::Error as LogicError;
use crate::secure::NodeSecret;

pub use crate::rest::{v0::logic, Context, FullContext};

pub async fn get_account_state(
    context: Data<Context>,
    account_id_hex: Path<String>,
) -> Result<impl Responder, Error> {
    logic::get_account_state(&context, &account_id_hex)
        .await
        .map_err(|err| match err {
            LogicError::PublicKey(e) => ErrorBadRequest(e),
            e => ErrorInternalServerError(e),
        })?
        .map(Json)
        .ok_or(ErrorNotFound("account not found"))
}

pub async fn get_message_logs(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_message_logs(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn post_message(context: Data<Context>, message: Bytes) -> Result<impl Responder, Error> {
    logic::post_message(&context, &message)
        .await
        .map(|()| HttpResponse::Ok().finish())
        .map_err(ErrorInternalServerError)
}

pub async fn get_tip(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_tip(&context)
        .await
        .map_err(ErrorInternalServerError)
}

pub async fn get_stats_counter(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_stats_counter(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_block_id(
    context: Data<Context>,
    block_id_hex: Path<String>,
) -> Result<impl Responder, Error> {
    logic::get_block_id(&context, &block_id_hex)
        .await
        .map_err(|err| match err {
            LogicError::Hash(e) => ErrorBadRequest(e),
            e => ErrorInternalServerError(e),
        })?
        .map(Bytes::from)
        .ok_or(ErrorNotFound("Block not found"))
}

pub async fn get_block_next_id(
    context: Data<Context>,
    block_id_hex: Path<String>,
    query_params: Query<QueryParams>,
) -> Result<impl Responder, Error> {
    logic::get_block_next_id(&context, &block_id_hex, query_params.get_count() as usize)
        .await
        .map_err(|err| match err {
            LogicError::Hash(e) => ErrorBadRequest(e),
            e => ErrorInternalServerError(e),
        })?
        .ok_or(ErrorNotFound("Block is not in chain of the tip"))
        .map(Bytes::from)
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
    logic::get_stake_distribution(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_stake_distribution_at(
    context: Data<Context>,
    epoch: Path<u32>,
) -> Result<impl Responder, Error> {
    logic::get_stake_distribution_at(&context, epoch.into_inner())
        .await
        .map_err(ErrorInternalServerError)?
        .map(Json)
        .ok_or(ErrorNotFound("Epoch not found"))
}

pub async fn get_settings(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_settings(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_shutdown(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_shutdown(&context)
        .await
        .map(|_| HttpResponse::Ok().finish())
        .map_err(ErrorInternalServerError)
}

pub async fn get_leaders(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_leader_ids(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn post_leaders(
    secret: Json<NodeSecret>,
    context: Data<Context>,
) -> Result<impl Responder, Error> {
    logic::post_leaders(&context, secret.into_inner())
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn delete_leaders(
    context: Data<Context>,
    leader_id: Path<EnclaveLeaderId>,
) -> Result<impl Responder, Error> {
    logic::delete_leaders(&context, leader_id.into_inner())
        .await
        .map_err(ErrorInternalServerError)?
        .map(|()| HttpResponse::Ok().finish())
        .ok_or(ErrorNotFound("Leader not found"))
}

pub async fn get_leaders_logs(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_leaders_logs(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_stake_pools(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_stake_pools(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_network_stats(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_network_stats(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_rewards_info_epoch(
    context: Data<Context>,
    epoch: Path<u32>,
) -> Result<impl Responder, Error> {
    logic::get_rewards_info_epoch(&context, epoch.into_inner())
        .await
        .map_err(ErrorInternalServerError)?
        .map(Json)
        .ok_or(ErrorNotFound(
            "Epoch not found or no rewards for this epoch",
        ))
}

pub async fn get_rewards_info_history(
    context: Data<Context>,
    length: Path<usize>,
) -> Result<impl Responder, Error> {
    logic::get_rewards_info_history(&context, length.into_inner())
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_utxo(
    context: Data<Context>,
    path_params: Path<(String, u8)>,
) -> Result<impl Responder, Error> {
    let (fragment_id_hex, output_index) = path_params.into_inner();
    logic::get_utxo(&context, &fragment_id_hex, output_index)
        .await
        .map_err(|err| match err {
            LogicError::Hash(e) => ErrorBadRequest(e),
            e => ErrorInternalServerError(e),
        })?
        .map(Json)
        .ok_or(ErrorNotFound("No UTXO found for address or index"))
}

pub async fn get_stake_pool(
    context: Data<Context>,
    pool_id_hex: Path<String>,
) -> Result<impl Responder, Error> {
    logic::get_stake_pool(&context, &pool_id_hex)
        .await
        .map_err(ErrorInternalServerError)?
        .map(Json)
        .ok_or(ErrorNotFound("Stake pool not found"))
}

pub async fn get_diagnostic(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_diagnostic(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_network_p2p_quarantined(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_network_p2p_quarantined(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_network_p2p_non_public(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_network_p2p_non_public(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_network_p2p_available(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_network_p2p_available(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_network_p2p_view(context: Data<Context>) -> Result<impl Responder, Error> {
    logic::get_network_p2p_view(&context)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}

pub async fn get_network_p2p_view_topic(
    context: Data<Context>,
    topic: Path<String>,
) -> Result<impl Responder, Error> {
    logic::get_network_p2p_view_topic(&context, &topic)
        .await
        .map(Json)
        .map_err(ErrorInternalServerError)
}
