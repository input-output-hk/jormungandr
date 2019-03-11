pub mod next_id;

use crate::rest::server_service::PathPredicate;
use actix_web::error::{Error as ActixError, ErrorBadRequest, ErrorInternalServerError};
use actix_web::{App, Path, State};
use blockcfg::mock::Mockchain;
use blockchain::BlockchainR;
use bytes::Bytes;
use chain_core::property::{FromStr, Serialize};
use chain_impl_mockchain::key::Hash;
use chain_storage::store::BlockStore;

pub fn create_handler(
    blockchain: BlockchainR<Mockchain>,
) -> impl Fn(&str) -> App<BlockchainR<Mockchain>> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        let path = format!("{}/v0/block/{{block_id}}", prefix);
        App::with_state(blockchain.clone())
            .filter(PathPredicate::for_pattern(&path))
            .resource(&path, |r| r.get().with(handle_request))
    }
}

fn handle_request(
    blockchain: State<BlockchainR<Mockchain>>,
    block_id_hex: Path<String>,
) -> Result<Bytes, ActixError> {
    let block_id = parse_block_hash(&block_id_hex)?;
    let block = blockchain
        .read()
        .unwrap()
        .storage
        .get_block(&block_id)
        .map_err(|e| ErrorBadRequest(e))?
        .0
        .serialize_as_vec()
        .map_err(|e| ErrorInternalServerError(e))?;
    Ok(Bytes::from(block))
}

fn parse_block_hash(hex: &str) -> Result<Hash, ActixError> {
    let block_hash = Hash::from_str(hex).map_err(|e| ErrorBadRequest(e))?;
    Ok(block_hash)
}
