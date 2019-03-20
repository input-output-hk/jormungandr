pub mod next_id;

use actix_web::error::{Error as ActixError, ErrorBadRequest, ErrorInternalServerError};
use actix_web::{App, Path, State};
use blockcfg::mock::Mockchain;
use blockchain::BlockchainR;
use bytes::Bytes;
use chain_core::property::Serialize;
use chain_crypto::Blake2b256;
use chain_impl_mockchain::key::Hash;

pub fn create_handler(
    blockchain: BlockchainR<Mockchain>,
) -> impl Fn(&str) -> App<BlockchainR<Mockchain>> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        App::with_state(blockchain.clone())
            .prefix(format!("{}/v0/block", prefix))
            .resource("/{block_id}", |r| r.get().with(handle_request))
            .resource("/{block_id}/next_id", |r| {
                r.get().with(next_id::handle_request)
            })
    }
}

fn handle_request(
    blockchain: State<BlockchainR<Mockchain>>,
    block_id_hex: Path<String>,
) -> Result<Bytes, ActixError> {
    let block_id = parse_block_hash(&block_id_hex)?;
    let blockchain = blockchain.read().unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_block_hash {
        use super::*;

        #[test]
        fn parses_valid_hex_encoded_hashes() {
            let hex = "000102030405060708090a0b0c0d0e0ff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff";

            let result = parse_block_hash(hex);

            let actual = result.unwrap();
            let expected = [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 240, 241, 242, 243, 244, 245,
                246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
            ];
            assert_eq!(expected, actual.as_ref());
        }

        #[test]
        fn rejects_invalid_hex_encoded_hashes() {
            let hex = "xx";

            let result = parse_block_hash(hex);

            let actual = result.unwrap_err();
            assert_eq!("invalid hex encoding for hash value", actual.to_string());
        }
    }
}
