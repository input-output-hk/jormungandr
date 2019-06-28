use blockchain::BlockchainR;
use jormungandr_lib::interfaces::*;

use actix_web::error::{Error, ErrorBadRequest, ErrorInternalServerError, ErrorNotFound};
use actix_web::{Error as ActixError, HttpMessage, HttpRequest};
use actix_web::{Json, Path, Responder, State, Query};
use chain_core::property::{Deserialize, Serialize};
use chain_crypto::{Blake2b256, PublicKey};
use chain_impl_mockchain::account::{AccountAlg, Identifier};
use chain_impl_mockchain::message::Message;
use chain_impl_mockchain::key::Hash;
use chain_storage::store;

use bytes::{Bytes, IntoBuf};
use futures::Future;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use crate::fragment::Logs;
use crate::intercom::TransactionMsg;
use crate::utils::async_msg::MessageBox;
use crate::rest::v0::node::stats::StatsCounter;


pub fn get_utxos(blockchain: State<BlockchainR>) -> impl Responder {
    let blockchain = blockchain.lock_read();
    let utxos = blockchain
        .multiverse
        .get(&blockchain.get_tip().unwrap())
        .unwrap()
        .utxos();
    let utxos = utxos.map(UTxOInfo::from).collect::<Vec<_>>();
    Json(utxos)
}

pub fn get_account_state(
    blockchain: State<BlockchainR>,
    account_id_hex: Path<String>,
) -> Result<impl Responder, Error> {
    let account_id = parse_account_id(&account_id_hex)?;
    let blockchain = blockchain.lock_read();
    let state = blockchain
        .multiverse
        .get(&blockchain.get_tip().unwrap())
        .unwrap()
        .accounts()
        .get_state(&account_id)
        .map_err(|e| ErrorNotFound(e))?;
    Ok(Json(AccountState::from(state)))
}

fn parse_account_id(id_hex: &str) -> Result<Identifier, Error> {
    PublicKey::<AccountAlg>::from_str(id_hex)
        .map(Into::into)
        .map_err(|e| ErrorBadRequest(e))
}

pub fn get_message_logs(logs: State<Arc<Mutex<Logs>>>) -> impl Responder {
    let logs = logs.lock().unwrap();
    let logs = logs.logs().wait().unwrap();
    Json(logs)
}

pub fn post_message(
    request: &HttpRequest<Arc<Mutex<MessageBox<TransactionMsg>>>>,
) -> impl Future<Item = impl Responder + 'static, Error = impl Into<ActixError> + 'static> + 'static
{
    let sender = request.state().clone();
    request.body().map(move |message| -> Result<_, ActixError> {
        let msg = Message::deserialize(message.into_buf()).map_err(|e| {
            println!("{}", e);
            ErrorBadRequest(e)
        })?;
        let msg = TransactionMsg::SendTransaction(FragmentOrigin::Rest, vec![msg]);
        sender.lock().unwrap().try_send(msg).unwrap();
        Ok("")
    })
}

pub fn get_tip(settings: State<BlockchainR>) -> impl Responder {
    settings.lock_read().get_tip().unwrap().to_string()
}

pub fn get_stats_counter(stats: State<StatsCounter>) -> impl Responder {
    Json(json!({
        "txRecvCnt": stats.get_tx_recv_cnt(),
        "blockRecvCnt": stats.get_block_recv_cnt(),
        "uptime": stats.get_uptime_sec(),
    }))
}

pub fn get_block_id(
    blockchain: State<BlockchainR>,
    block_id_hex: Path<String>,
) -> Result<Bytes, ActixError> {
    let block_id = parse_block_hash(&block_id_hex)?;
    let blockchain = blockchain.lock_read();
    let block = blockchain
        .storage
        .read()
        .unwrap()
        .get_block(&block_id)
        .map_err(|e| ErrorBadRequest(e))?
        .0
        .serialize_as_vec()
        .map_err(|e| ErrorInternalServerError(e))?;
    Ok(Bytes::from(block))
}

fn parse_block_hash(hex: &str) -> Result<Hash, ActixError> {
    let hash: Blake2b256 = hex.parse().map_err(|e| ErrorBadRequest(e))?;
    Ok(Hash::from(hash))
}


pub fn get_block_next_id(
    blockchain: State<BlockchainR>,
    block_id_hex: Path<String>,
    query_params: Query<QueryParams>,
) -> Result<Bytes, ActixError> {
    let block_id = parse_block_hash(&block_id_hex)?;
    // FIXME
    // POSSIBLE RACE CONDITION OR DEADLOCK!
    // Assuming that during update whole blockchain is write-locked
    // FIXME: don't hog the blockchain lock.
    let blockchain = blockchain.lock_read();
    let storage = blockchain.storage.read().unwrap();
    store::iterate_range(&*storage, &block_id, &blockchain.get_tip().unwrap())
        .map_err(|e| ErrorBadRequest(e))?
        .take(query_params.get_count())
        .try_fold(Bytes::new(), |mut bytes, res| {
            let block_info = res.map_err(|e| ErrorInternalServerError(e))?;
            bytes.extend_from_slice(block_info.block_hash.as_ref());
            Ok(bytes)
        })
}

const MAX_COUNT: usize = 100;

#[derive(Deserialize)]
pub struct QueryParams {
    count: Option<usize>,
}

impl QueryParams {
    pub fn get_count(&self) -> usize {
        self.count.unwrap_or(1).min(MAX_COUNT)
    }
}