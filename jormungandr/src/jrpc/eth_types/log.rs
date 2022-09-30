use super::{bytes::Bytes, number::Number};
use chain_evm::ethereum_types::{H160, H256};
use serde::Serialize;

/// Log
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Log {
    /// Whether Log Type is Removed (Geth Compatibility Field)
    removed: bool,
    /// Log Index in Block
    log_index: Option<Number>,
    /// Transaction Index
    transaction_index: Option<Number>,
    /// Transaction Hash
    transaction_hash: Option<H256>,
    /// Block Hash
    block_hash: Option<H256>,
    /// Block Number
    block_number: Option<Number>,
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
            log_index: Some(1.into()),
            transaction_index: Some(1.into()),
            transaction_hash: Some(H256::zero()),
            block_hash: Some(H256::zero()),
            block_number: Some(1.into()),
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
            log_index: Some(0.into()),
            transaction_index: Some(0.into()),
            transaction_hash: Some(H256::zero()),
            block_hash: Some(H256::zero()),
            block_number: Some(0.into()),
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
