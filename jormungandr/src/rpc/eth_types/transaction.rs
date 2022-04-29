use chain_evm::ethereum_types::{H160, U256, U64};
use chain_impl_mockchain::evm::EvmTransaction;
use serde::Serialize;

use super::block::Bytes;

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Nonce
    pub nonce: U256,
    /// Sender
    pub from: H160,
    /// Recipient
    pub to: Option<H160>,
    /// Transfered value
    pub value: U256,
    /// Gas
    pub gas: U256,
    /// Data
    pub input: Bytes,
    #[serde(rename = "gasPrice", skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<U256>,
    /// The network id of the transaction, if any.
    pub chain_id: Option<U64>,
    /// The standardised V field of the signature.
    pub v: U256,
    /// The R field of the signature.
    pub r: U256,
    /// The S field of the signature.
    pub s: U256,
    /// EIP-2718 type
    #[serde(rename = "type")]
    pub transaction_type: U256,
}

impl Transaction {
    pub fn build(tx: EvmTransaction) -> Self {
        match tx {
            EvmTransaction::Call {
                caller,
                address,
                value,
                data,
                gas_limit,
                ..
            } => Transaction {
                nonce: U256::zero(),
                from: caller,
                to: Some(address),
                value,
                gas_price: None,
                gas: gas_limit.into(),
                input: data.into(),
                chain_id: None,
                v: Default::default(),
                r: Default::default(),
                s: Default::default(),
                transaction_type: U256::zero(),
            },
            EvmTransaction::Create {
                caller,
                value,
                init_code,
                gas_limit,
                ..
            } => Transaction {
                nonce: U256::zero(),
                from: caller,
                to: None,
                value,
                gas_price: None,
                gas: gas_limit.into(),
                input: init_code.into(),
                chain_id: None,
                v: Default::default(),
                r: Default::default(),
                s: Default::default(),
                transaction_type: U256::zero(),
            },
            EvmTransaction::Create2 {
                caller,
                value,
                init_code,
                gas_limit,
                ..
            } => Transaction {
                nonce: U256::zero(),
                from: caller,
                to: None,
                value,
                gas_price: None,
                gas: gas_limit.into(),
                input: init_code.into(),
                chain_id: None,
                v: Default::default(),
                r: Default::default(),
                s: Default::default(),
                transaction_type: U256::zero(),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn block_json_test() {
        let h160 = H160::from_slice(&[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
        ]);
        let u256 = U256::one();
        let u64 = U64::one();
        let bytes: Bytes = vec![1, 2, 3].into();

        let transaction = Transaction {
            nonce: u256,
            from: h160,
            to: Some(h160),
            value: u256,
            gas_price: Some(u256),
            gas: u256,
            input: bytes.clone(),
            chain_id: Some(u64),
            v: u256,
            r: u256,
            s: u256,
            transaction_type: u256,
        };

        let json = serde_json::to_string(&transaction).unwrap();

        assert_eq!(json, "{\"hash\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"nonce\":\"0x1\",\"blockHash\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"blockNumber\":\"0x1\",\"transactionIndex\":\"0x1\",\"from\":\"0x000102030405060708090a0b0c0d0e0f10111213\",\"to\":\"0x000102030405060708090a0b0c0d0e0f10111213\",\"value\":\"0x1\",\"gasPrice\":\"0x1\",\"maxFeePerGas\":\"0x1\",\"maxPriorityFeePerGas\":\"0x1\",\"gas\":\"0x1\",\"input\":\"0x010203\",\"creates\":\"0x000102030405060708090a0b0c0d0e0f10111213\",\"raw\":\"0x010203\",\"publicKey\":\"0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f\",\"chainId\":\"0x1\",\"standardV\":\"0x1\",\"v\":\"0x1\",\"r\":\"0x1\",\"s\":\"0x1\",\"type\":\"0x1\"}");
    }
}
