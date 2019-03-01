use actix_web::error::{ErrorBadRequest, ErrorInternalServerError};
use actix_web::{App, Path, Responder, State};
use blockcfg::mock::Mockchain;
use blockchain::BlockchainR;
use bytes::Bytes;
use chain_core::property::Serialize;
use chain_impl_mockchain::key::Hash;
use chain_storage::store::BlockStore;
use serde::de::{Deserialize, Deserializer, Error};

pub fn create_handler(
    blockchain: BlockchainR<Mockchain>,
) -> impl Fn(&str) -> App<BlockchainR<Mockchain>> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        App::with_state(blockchain.clone())
            .prefix(format!("{}/v0/block", prefix))
            .resource("{block_id}", |r| r.get().with(handle_request))
    }
}

fn handle_request(
    blockchain: State<BlockchainR<Mockchain>>,
    block_id_hex: Path<String>,
) -> impl Responder {
    let block_id = match block_id_hex.parse() {
        Ok(block_id) => block_id,
        Err(e) => return Err(ErrorBadRequest(e)),
    };
    let block = blockchain
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .storage
        .get_block(&block_id)
        .map_err(|e| ErrorBadRequest(e))?
        .0
        .serialize_as_vec()
        .map_err(|e| ErrorInternalServerError(e))?;
    Ok(Bytes::from(block))
}

#[derive(Deserialize)]
struct BlockPath {
    #[serde(deserialize_with = "deserialize_hash")]
    pub block_id: Hash,
}

fn deserialize_hash<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Hash, D::Error> {
    let hash_hex = <&str>::deserialize(deserializer)?;
    hash_hex.parse().map_err(D::Error::custom)
}
