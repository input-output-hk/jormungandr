use chain_evm::ethereum_types::{H160, H256, H512, U256, U64};
use chain_impl_mockchain::evm::EvmTransaction;
use serde::Serialize;

use super::block::Bytes;

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Hash
    pub hash: H256,
    /// Nonce
    pub nonce: U256,
    /// Block hash
    pub block_hash: H256,
    /// Block number
    pub block_number: U256,
    /// Transaction Index
    pub transaction_index: Option<U256>,
    /// Sender
    pub from: H160,
    /// Recipient
    pub to: Option<H160>,
    /// Transfered value
    pub value: U256,
    /// Gas Price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<U256>,
    /// Max BaseFeePerGas the user is willing to pay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_gas: Option<U256>,
    /// The miner's tip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_priority_fee_per_gas: Option<U256>,
    /// Gas
    pub gas: U256,
    /// Data
    pub input: Bytes,
    /// Creates contract
    pub creates: Option<H160>,
    /// Raw transaction data
    pub raw: Bytes,
    /// Public key of the signer.
    pub public_key: Option<H512>,
    /// The network id of the transaction, if any.
    pub chain_id: Option<U64>,
    /// The standardised V field of the signature (0 or 1).
    pub standard_v: U256,
    /// The standardised V field of the signature.
    pub v: U256,
    /// The R field of the signature.
    pub r: U256,
    /// The S field of the signature.
    pub s: U256,
    // /// Pre-pay to warm storage access.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub access_list: Option<Vec<AccessListItem>>,
    /// EIP-2718 type
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<U256>,
}

impl Transaction {
    pub fn build(block_hash: H256, block_number: U256, hash: H256, tx: EvmTransaction) -> Self {
        match tx {
            EvmTransaction::Call {
                caller,
                address,
                value,
                data,
                gas_limit,
                ..
            } => Transaction {
                hash,
                nonce: Default::default(),
                block_hash,
                block_number,
                transaction_index: Some(Default::default()),
                from: caller,
                to: Some(address),
                value,
                gas_price: None,
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
                gas: gas_limit.into(),
                input: data.into(),
                creates: None,
                raw: Default::default(),
                public_key: None,
                chain_id: None,
                standard_v: Default::default(),
                v: Default::default(),
                r: Default::default(),
                s: Default::default(),
                transaction_type: None,
            },
            EvmTransaction::Create {
                caller,
                value,
                init_code,
                gas_limit,
                ..
            } => Transaction {
                hash,
                nonce: Default::default(),
                block_hash,
                block_number,
                transaction_index: Some(Default::default()),
                from: caller,
                to: None,
                value,
                gas_price: None,
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
                gas: gas_limit.into(),
                input: init_code.into(),
                creates: None,
                raw: Default::default(),
                public_key: None,
                chain_id: None,
                standard_v: Default::default(),
                v: Default::default(),
                r: Default::default(),
                s: Default::default(),
                transaction_type: None,
            },
            EvmTransaction::Create2 {
                caller,
                value,
                init_code,
                gas_limit,
                ..
            } => Transaction {
                hash,
                nonce: Default::default(),
                block_hash,
                block_number,
                transaction_index: Some(Default::default()),
                from: caller,
                to: None,
                value,
                gas_price: None,
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
                gas: gas_limit.into(),
                input: init_code.into(),
                creates: None,
                raw: Default::default(),
                public_key: None,
                chain_id: None,
                standard_v: Default::default(),
                v: Default::default(),
                r: Default::default(),
                s: Default::default(),
                transaction_type: None,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn block_json_test() {
        let h512 = H512::from_slice(&[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ]);
        let h256 = H256::from_slice(&[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]);
        let h160 = H160::from_slice(&[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
        ]);
        let u256 = U256::one();
        let u64 = U64::one();
        let bytes: Bytes = vec![1, 2, 3].into();

        let transaction = Transaction {
            hash: h256,
            nonce: u256,
            block_hash: h256,
            block_number: u256,
            transaction_index: Some(u256),
            from: h160,
            to: Some(h160),
            value: u256,
            gas_price: Some(u256),
            max_fee_per_gas: Some(u256),
            max_priority_fee_per_gas: Some(u256),
            gas: u256,
            input: bytes.clone(),
            creates: Some(h160),
            raw: bytes,
            public_key: Some(h512),
            chain_id: Some(u64),
            standard_v: u256,
            v: u256,
            r: u256,
            s: u256,
            transaction_type: Some(u256),
        };

        let json = serde_json::to_string(&transaction).unwrap();

        assert_eq!(json, "{\"hash\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"nonce\":\"0x1\",\"blockHash\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"blockNumber\":\"0x1\",\"transactionIndex\":\"0x1\",\"from\":\"0x000102030405060708090a0b0c0d0e0f10111213\",\"to\":\"0x000102030405060708090a0b0c0d0e0f10111213\",\"value\":\"0x1\",\"gasPrice\":\"0x1\",\"maxFeePerGas\":\"0x1\",\"maxPriorityFeePerGas\":\"0x1\",\"gas\":\"0x1\",\"input\":\"0x010203\",\"creates\":\"0x000102030405060708090a0b0c0d0e0f10111213\",\"raw\":\"0x010203\",\"publicKey\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f\",\"chainId\":\"0x1\",\"standardV\":\"0x1\",\"v\":\"0x1\",\"r\":\"0x1\",\"s\":\"0x1\",\"type\":\"0x1\"}");
    }
}
