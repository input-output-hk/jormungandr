use super::{bytes::Bytes, number::Number, transaction::Transaction};
use chain_core::property::Serialize;
use chain_evm::ethereum_types::{Bloom, H160, H256};
use chain_impl_mockchain::{
    block::{Block as JorBlock, Header as JorHeader},
    fragment::Fragment,
};

/// Block Transactions
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum BlockTransactions {
    /// Only hashes
    Hashes(Vec<H256>),
    /// Full transactions
    Full(Vec<Transaction>),
}

/// Block header representation.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    /// Hash of the block
    hash: H256,
    /// Mix Hash of the block
    mix_hash: H256,
    /// Nonce of the block,
    nonce: Number,
    /// Hash of the parent
    parent_hash: H256,
    /// Hash of the uncles
    #[serde(rename = "sha3Uncles")]
    uncles_hash: H256,
    /// Alias of `author`
    miner: H160,
    /// State root hash (same as transactions_root)
    state_root: H256,
    /// Transactions root hash,
    transactions_root: H256,
    /// Transactions receipts root hash
    receipts_root: H256,
    /// Block number
    number: Number,
    /// Gas Used
    gas_used: Number,
    /// Gas Limit
    gas_limit: Number,
    /// Extra data
    extra_data: Bytes,
    /// Logs bloom
    logs_bloom: Bloom,
    /// Timestamp
    timestamp: Number,
    /// Difficulty
    difficulty: Option<Number>,
}

impl Header {
    pub fn build(header: JorHeader) -> Self {
        Self {
            hash: H256::from_slice(header.hash().as_ref()),
            mix_hash: H256::zero(),
            nonce: 1.into(),
            parent_hash: H256::from_slice(header.block_parent_hash().as_ref()),
            uncles_hash: H256::zero(),
            miner: header
                .get_bft_leader_id()
                .map(|id| H160::from_slice(id.as_ref()))
                .unwrap_or_else(|| H160::zero()),
            state_root: H256::zero(),
            transactions_root: H256::from_slice(header.block_content_hash().as_ref()),
            receipts_root: H256::zero(),
            number: Into::<u64>::into(Into::<u32>::into(header.chain_length())).into(),
            gas_used: 0.into(),
            gas_limit: 0.into(),
            extra_data: Bytes::default(),
            logs_bloom: Bloom::zero(),
            timestamp: 0.into(),
            difficulty: Some(0.into()),
        }
    }
}

/// Block representation
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    /// Header of the block
    #[serde(flatten)]
    header: Header,
    /// Total difficulty
    total_difficulty: Number,
    /// Uncles' hashes
    uncles: Vec<H256>,
    /// Transactions
    transactions: BlockTransactions,
    /// Size in bytes
    size: Number,
    /// Base Fee for post-EIP1559 blocks.
    base_fee_per_gas: Option<Number>,
}

impl Block {
    pub fn build(block: JorBlock, full: bool) -> Self {
        let header = Header::build(block.header().clone());
        let transactions = if full {
            let mut res = Vec::new();
            for fragment in block.fragments() {
                if let Fragment::Evm(evm_tx) = fragment {
                    let _evm_tx = evm_tx.as_slice().payload().into_payload();
                    res.push(Transaction::build());
                }
            }
            BlockTransactions::Full(res)
        } else {
            let mut res = Vec::new();
            for fragment in block.fragments() {
                if let Fragment::Evm(evm_tx) = fragment {
                    res.push(H256::from_slice(evm_tx.hash().as_ref()));
                }
            }
            BlockTransactions::Hashes(res)
        };

        Self {
            header,
            total_difficulty: 0.into(),
            uncles: Default::default(),
            transactions,
            size: (block.serialized_size() as u64).into(),
            base_fee_per_gas: Some(1.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_serialize() {
        let header = Header {
            hash: H256::zero(),
            mix_hash: H256::zero(),
            nonce: 0.into(),
            parent_hash: H256::zero(),
            uncles_hash: H256::zero(),
            miner: H160::zero(),
            state_root: H256::zero(),
            transactions_root: H256::zero(),
            receipts_root: H256::zero(),
            number: 0.into(),
            gas_used: 0.into(),
            gas_limit: 0.into(),
            extra_data: Bytes::default(),
            logs_bloom: Bloom::zero(),
            timestamp: 0.into(),
            difficulty: Some(0.into()),
        };

        let block = Block {
            header: header,
            total_difficulty: 0.into(),
            uncles: Default::default(),
            transactions: BlockTransactions::Hashes(vec![H256::zero()]),
            size: 0.into(),
            base_fee_per_gas: Some(0.into()),
        };

        assert_eq!(
            serde_json::to_string(&block).unwrap(),
            r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000000","mixHash":"0x0000000000000000000000000000000000000000000000000000000000000000","nonce":"0x0","parentHash":"0x0000000000000000000000000000000000000000000000000000000000000000","sha3Uncles":"0x0000000000000000000000000000000000000000000000000000000000000000","miner":"0x0000000000000000000000000000000000000000","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionsRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","receiptsRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","number":"0x0","gasUsed":"0x0","gasLimit":"0x0","extraData":"0x","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","timestamp":"0x0","difficulty":"0x0","totalDifficulty":"0x0","uncles":[],"transactions":["0x0000000000000000000000000000000000000000000000000000000000000000"],"size":"0x0","baseFeePerGas":"0x0"}"#
        );
    }
}
