use crate::rest::server_service::PathPredicate;
use actix_web::error::{Error as ActixError, ErrorBadRequest, ErrorInternalServerError};
use actix_web::{App, Path, Query, State};
use blockcfg::mock::Mockchain;
use blockchain::BlockchainR;
use bytes::Bytes;
use chain_core::property::Settings;
use chain_storage::store::BlockStore;

pub fn create_handler(
    blockchain: BlockchainR<Mockchain>,
) -> impl Fn(&str) -> App<BlockchainR<Mockchain>> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        let path = format!("{}/v0/block/{{block_id}}/next_id", prefix);
        App::with_state(blockchain.clone())
            .filter(PathPredicate::for_pattern(&path))
            .resource(&path, |r| r.get().with(handle_request))
    }
}

fn handle_request(
    blockchain: State<BlockchainR<Mockchain>>,
    block_id_hex: Path<String>,
    query_params: Query<QueryParams>,
) -> Result<Bytes, ActixError> {
    let block_id = block_id_hex.parse().map_err(|e| ErrorBadRequest(e))?;
    // FIXME
    // POSSIBLE RACE CONDITION OR DEADLOCK!
    // Assuming that during update whole blockchain is write-locked
    let blockchain = blockchain.read().unwrap();
    let tip = blockchain.state.settings.read().unwrap().tip();
    blockchain
        .storage
        .iterate_range(&block_id, &tip)
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
struct QueryParams {
    count: Option<usize>,
}

impl QueryParams {
    pub fn get_count(&self) -> usize {
        self.count.unwrap_or(1).min(MAX_COUNT)
    }
}
