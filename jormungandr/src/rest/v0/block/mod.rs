use actix_web::App;
use blockchain::BlockchainR;

use crate::rest::v0::handlers;

pub fn create_handler(
    blockchain: BlockchainR,
) -> impl Fn(&str) -> App<BlockchainR> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        App::with_state(blockchain.clone())
            .prefix(format!("{}/v0/block", prefix))
            .resource("/{block_id}", |r| r.get().with(handlers::get_block_id))
            .resource("/{block_id}/next_id", |r| {
                r.get().with(handlers::get_block_next_id)
            })
    }
}

/*
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

*/