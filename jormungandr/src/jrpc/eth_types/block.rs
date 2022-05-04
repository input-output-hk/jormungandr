use super::{bytes::Bytes, transaction::Transaction};
use chain_evm::ethereum_types::{Bloom, H160, H256, U256};
use serde::{Serialize, Serializer};

/// Block Transactions
#[derive(Debug)]
pub enum BlockTransactions {
    /// Only hashes
    Hashes(Vec<H256>),
    /// Full transactions
    Full(Vec<Transaction>),
}

impl Default for BlockTransactions {
    fn default() -> Self {
        Self::Hashes(Vec::new())
    }
}

impl Serialize for BlockTransactions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            BlockTransactions::Hashes(ref hashes) => hashes.serialize(serializer),
            BlockTransactions::Full(ref ts) => ts.serialize(serializer),
        }
    }
}

/// Block header representation.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    /// Hash of the block
    hash: H256,
    /// Mix Hash of the block
    mix_hash: H256,
    /// Nonce of the block,
    nonce: U256,
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
    number: U256,
    /// Gas Used
    gas_used: U256,
    /// Gas Limit
    gas_limit: U256,
    /// Extra data
    extra_data: Bytes,
    /// Logs bloom
    logs_bloom: Bloom,
    /// Timestamp
    timestamp: U256,
    /// Difficulty
    difficulty: Option<U256>,
}

impl Header {
    pub fn build() -> Self {
        Self {
            hash: H256::zero(),
            mix_hash: H256::zero(),
            nonce: U256::one(),
            parent_hash: H256::zero(),
            uncles_hash: H256::zero(),
            miner: H160::zero(),
            state_root: H256::zero(),
            transactions_root: H256::zero(),
            receipts_root: H256::zero(),
            number: U256::one(),
            gas_used: U256::one(),
            gas_limit: U256::one(),
            extra_data: Bytes::default(),
            logs_bloom: Bloom::zero(),
            timestamp: U256::one(),
            difficulty: Some(U256::one()),
        }
    }
}

/// Block representation
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    /// Header of the block
    #[serde(flatten)]
    header: Header,
    /// Total difficulty
    total_difficulty: U256,
    /// Uncles' hashes
    uncles: Vec<H256>,
    /// Transactions
    transactions: BlockTransactions,
    /// Size in bytes
    size: U256,
    /// Base Fee for post-EIP1559 blocks.
    base_fee_per_gas: Option<U256>,
}

impl Block {
    pub fn build(full: bool) -> Self {
        let header = Header::build();
        let transactions = if full {
            BlockTransactions::Full(vec![Transaction::build()])
        } else {
            BlockTransactions::Hashes(vec![H256::zero()])
        };

        Self {
            header,
            total_difficulty: U256::one(),
            uncles: Default::default(),
            transactions,
            size: U256::one(),
            base_fee_per_gas: Some(U256::one()),
        }
    }
}
