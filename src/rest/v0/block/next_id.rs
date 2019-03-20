use super::parse_block_hash;
use actix_web::error::{Error as ActixError, ErrorBadRequest, ErrorInternalServerError};
use actix_web::{Path, Query, State};
use blockcfg::mock::Mockchain;
use blockchain::BlockchainR;
use bytes::Bytes;
use chain_core::property::Settings;

pub fn handle_request(
    blockchain: State<BlockchainR<Mockchain>>,
    block_id_hex: Path<String>,
    query_params: Query<QueryParams>,
) -> Result<Bytes, ActixError> {
    let block_id = parse_block_hash(&block_id_hex)?;
    // FIXME
    // POSSIBLE RACE CONDITION OR DEADLOCK!
    // Assuming that during update whole blockchain is write-locked
    let blockchain = blockchain.read().unwrap();
    let tip = blockchain.state.tip();
    let storage = blockchain.storage.read().unwrap();
    storage
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
pub struct QueryParams {
    count: Option<usize>,
}

impl QueryParams {
    pub fn get_count(&self) -> usize {
        self.count.unwrap_or(1).min(MAX_COUNT)
    }
}
