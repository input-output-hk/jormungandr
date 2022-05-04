use super::transaction::Transaction;
use chain_evm::ethereum_types::{Bloom, H160, H256, U256};
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bytes(Box<[u8]>);

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialized = "0x".to_owned();
        serialized.push_str(hex::encode(&self.0).as_str());
        serializer.serialize_str(serialized.as_ref())
    }
}

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
    pub hash: H256,
    /// Mix Hash of the block
    pub mix_hash: H256,
    /// Nonce of the block,
    pub nonce: U256,
    /// Hash of the parent
    pub parent_hash: H256,
    /// Hash of the uncles
    #[serde(rename = "sha3Uncles")]
    pub uncles_hash: H256,
    /// Alias of `author`
    pub miner: H160,
    /// State root hash (same as transactions_root)
    pub state_root: H256,
    /// Transactions root hash,
    pub transactions_root: H256,
    /// Transactions receipts root hash
    pub receipts_root: H256,
    /// Block number
    pub number: U256,
    /// Gas Used
    pub gas_used: U256,
    /// Gas Limit
    pub gas_limit: U256,
    /// Extra data
    pub extra_data: Bytes,
    /// Logs bloom
    pub logs_bloom: Bloom,
    /// Timestamp
    pub timestamp: U256,
    /// Difficulty
    pub difficulty: Option<U256>,
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
    pub header: Header,
    /// Total difficulty
    pub total_difficulty: U256,
    /// Uncles' hashes
    pub uncles: Vec<H256>,
    /// Transactions
    pub transactions: BlockTransactions,
    /// Size in bytes
    pub size: U256,
    /// Base Fee for post-EIP1559 blocks.
    pub base_fee_per_gas: Option<U256>,
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
