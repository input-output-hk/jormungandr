use super::{bytes::Bytes, number::Number};
use chain_evm::ethereum_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Block hash, null when pending
    block_hash: Option<H256>,
    /// Block number, null when pending
    block_number: Option<Number>,
    /// Nonce
    nonce: Number,
    /// Sender
    from: H160,
    /// Recipient
    to: Option<H160>,
    /// Transfered value
    value: Number,
    /// Gas
    gas: Number,
    /// Data
    input: Bytes,
    /// Gas price
    gas_price: Number,
    /// The network id of the transaction, if any.
    chain_id: Option<Number>,
    /// Transaction Index, null when pending
    transaction_index: Option<Number>,
    /// The standardised V field of the signature.
    v: Number,
    /// The R field of the signature.
    r: U256,
    /// The S field of the signature.
    s: U256,
    /// EIP-2718 type
    #[serde(rename = "type")]
    transaction_type: Number,
}

impl Transaction {
    pub fn build() -> Self {
        Self {
            block_hash: None,
            block_number: None,
            nonce: 1.into(),
            from: H160::zero(),
            to: Some(H160::zero()),
            value: 1.into(),
            gas: 1.into(),
            input: Default::default(),
            gas_price: 1.into(),
            chain_id: Some(1.into()),
            transaction_index: None,
            v: 1.into(),
            r: U256::one(),
            s: U256::one(),
            transaction_type: 1.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_json_serde() {
        let transaction = Transaction {
            block_hash: None,
            block_number: None,
            nonce: 0.into(),
            from: H160::zero(),
            to: Some(H160::zero()),
            value: 0.into(),
            gas: 0.into(),
            input: Default::default(),
            gas_price: 0.into(),
            chain_id: Some(0.into()),
            transaction_index: None,
            v: 0.into(),
            r: U256::zero(),
            s: U256::zero(),
            transaction_type: 0.into(),
        };
        assert_eq!(
            serde_json::to_string(&transaction).unwrap(),
            r#"{"blockHash":null,"blockNumber":null,"nonce":"0x0","from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","value":"0x0","gas":"0x0","input":"0x","gasPrice":"0x0","chainId":"0x0","transactionIndex":null,"v":"0x0","r":"0x0","s":"0x0","type":"0x0"}"#
        );
        let decoded: Transaction = serde_json::from_str(r#"{"blockHash":null,"blockNumber":null,"nonce":"0x0","from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","value":"0x0","gas":"0x0","input":"0x","gasPrice":"0x0","chainId":"0x0","transactionIndex":null,"v":"0x0","r":"0x0","s":"0x0","type":"0x0"}"#
    ).unwrap();
        assert_eq!(decoded, transaction);
    }
}
