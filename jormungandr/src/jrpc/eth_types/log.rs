use super::bytes::Bytes;
use chain_evm::ethereum_types::{H160, H256, U256};
use serde::Serialize;

/// Log
#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Log {
    /// Whether Log Type is Removed (Geth Compatibility Field)
    removed: bool,
    /// Log Index in Block
    log_index: Option<U256>,
    /// Transaction Index
    transaction_index: Option<U256>,
    /// Transaction Hash
    transaction_hash: Option<H256>,
    /// Block Hash
    block_hash: Option<H256>,
    /// Block Number
    block_number: Option<U256>,
    /// H160
    address: Option<H160>,
    /// Data
    data: Option<Bytes>,
    /// Topics
    topics: Vec<H256>,
}

impl Log {
    pub fn build() -> Self {
        Self {
            removed: true,
            log_index: Some(U256::zero()),
            transaction_index: Some(U256::zero()),
            transaction_hash: Some(H256::zero()),
            block_hash: Some(H256::zero()),
            block_number: Some(U256::zero()),
            address: Some(H160::zero()),
            data: Some(Default::default()),
            topics: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_json_serialize() {
        let log = Log {
            removed: true,
            log_index: Some(U256::zero()),
            transaction_index: Some(U256::zero()),
            transaction_hash: Some(H256::zero()),
            block_hash: Some(H256::zero()),
            block_number: Some(U256::zero()),
            address: Some(H160::zero()),
            data: Some(Default::default()),
            topics: Default::default(),
        };
        assert_eq!(
            serde_json::to_string(&log).unwrap(),
            r#"{"removed":true,"logIndex":"0x0","transactionIndex":"0x0","transactionHash":"0x0000000000000000000000000000000000000000000000000000000000000000","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000000","blockNumber":"0x0","address":"0x0000000000000000000000000000000000000000","data":"0x","topics":[]}"#
        );
    }
}
