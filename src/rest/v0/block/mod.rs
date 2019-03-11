pub mod next_id;

use actix_web::error::{Error as ActixError, ErrorBadRequest, ErrorInternalServerError};
use actix_web::{App, Path, State};
use blockcfg::mock::Mockchain;
use blockchain::BlockchainR;
use bytes::Bytes;
use chain_core::property::Serialize;
use chain_crypto::Blake2b256;
use chain_impl_mockchain::key::Hash;
use chain_storage::store::BlockStore;
use hex::FromHex;

pub fn create_handler(
    blockchain: BlockchainR<Mockchain>,
) -> impl Fn(&str) -> App<BlockchainR<Mockchain>> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        App::with_state(blockchain.clone())
            .prefix(prefix.to_string())
            .resource("/v0/block/{{block_id}}", |r| r.get().with(handle_request))
            .resource("/v0/block/{{block_id}}/next_id", |r| {
                r.get().with(next_id::handle_request)
            })
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
    let bytes = <[u8; Blake2b256::HASH_SIZE]>::from_hex(hex).map_err(|e| ErrorBadRequest(e))?;
    let hash = Blake2b256::from(bytes);
    let block_hash = Hash::from(hash);
    Ok(block_hash)
}
