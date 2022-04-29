use super::transaction::Transaction;
use chain_core::property::Serialize as _;
use chain_evm::ethereum_types::{Bloom, H160, H256, U256};
use chain_impl_mockchain::fragment::Fragment;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bytes(Vec<u8>);

impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Bytes {
        Bytes(bytes)
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(val: Bytes) -> Self {
        val.0
    }
}

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
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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
    pub fn build(val: chain_impl_mockchain::block::Header) -> Self {
        Self {
            hash: H256::from_slice(val.hash().as_bytes()),
            mix_hash: H256::zero(),
            nonce: U256::zero(),
            parent_hash: H256::from_slice(val.block_parent_hash().as_bytes()),
            uncles_hash: H256::zero(),
            miner: H160::zero(),
            state_root: H256::from_slice(val.block_content_hash().as_bytes()),
            transactions_root: H256::from_slice(val.block_content_hash().as_bytes()),
            receipts_root: H256::zero(),
            number: (<u32>::from(val.chain_length())).into(),
            gas_used: U256::zero(),
            gas_limit: U256::zero(),
            extra_data: Bytes::default(),
            logs_bloom: Bloom::zero(),
            timestamp: U256::zero(),
            difficulty: Some(U256::zero()),
        }
    }
}

/// Block representation
#[derive(Debug, Serialize)]
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
    pub fn build(block: chain_impl_mockchain::block::Block, full: bool) -> Self {
        let header = Header::build(block.header().clone());
        let transactions = match full {
            true => BlockTransactions::Hashes(
                block
                    .fragments()
                    .map(|tx| H256::from_slice(tx.hash().as_bytes()))
                    .collect(),
            ),
            false => {
                let mut txs = Vec::new();
                for tx in block.fragments() {
                    if let Fragment::Evm(tx) = tx {
                        let tx = tx.as_slice().payload().into_payload();
                        txs.push(Transaction::build(tx))
                    }
                }
                BlockTransactions::Full(txs)
            }
        };

        Self {
            header,
            total_difficulty: U256::zero(),
            uncles: Default::default(),
            transactions,
            size: block.serialized_size().into(),
            base_fee_per_gas: None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn block_json_test() {
        let h256 = H256::from_slice(&[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]);
        let h160 = H160::from_slice(&[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
        ]);
        let u256 = U256::one();
        let bloom = Bloom::default();
        let bytes: Bytes = vec![1, 2, 3].into();

        let header = Header {
            hash: h256,
            mix_hash: h256,
            nonce: u256,
            parent_hash: h256,
            uncles_hash: h256,
            miner: h160,
            state_root: h256,
            transactions_root: h256,
            receipts_root: h256,
            number: u256,
            gas_used: u256,
            gas_limit: u256,
            extra_data: bytes.clone(),
            logs_bloom: bloom,
            timestamp: u256,
            difficulty: Some(u256),
        };

        let json = serde_json::to_string(&header).unwrap();

        assert_eq!(
            json,
            "{\"hash\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"mixHash\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"nonce\":\"0x1\",\"parentHash\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"sha3Uncles\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"miner\":\"0x000102030405060708090a0b0c0d0e0f10111213\",\"stateRoot\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"transactionsRoot\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"receiptsRoot\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"number\":\"0x1\",\"gasUsed\":\"0x1\",\"gasLimit\":\"0x1\",\"extraData\":\"0x010203\",\"logsBloom\":\"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\",\"timestamp\":\"0x1\",\"difficulty\":\"0x1\"}"
        );

        let block = Block {
            header,
            total_difficulty: u256,
            uncles: vec![h256],
            transactions: BlockTransactions::Hashes(vec![h256]),
            size: u256,
            base_fee_per_gas: Some(u256),
        };

        let json = serde_json::to_string(&block).unwrap();

        assert_eq!(
            json,
            "{\"hash\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"mixHash\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"nonce\":\"0x1\",\"parentHash\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"sha3Uncles\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"miner\":\"0x000102030405060708090a0b0c0d0e0f10111213\",\"stateRoot\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"transactionsRoot\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"receiptsRoot\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"number\":\"0x1\",\"gasUsed\":\"0x1\",\"gasLimit\":\"0x1\",\"extraData\":\"0x010203\",\"logsBloom\":\"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\",\"timestamp\":\"0x1\",\"difficulty\":\"0x1\",\"totalDifficulty\":\"0x1\",\"uncles\":[\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\"],\"transactions\":[\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\"],\"size\":\"0x1\",\"baseFeePerGas\":\"0x1\"}"
        );
    }
}
