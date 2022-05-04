use super::bytes::Bytes;
use chain_evm::ethereum_types::{H160, U256, U64};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Nonce
    nonce: U256,
    /// Sender
    from: H160,
    /// Recipient
    to: Option<H160>,
    /// Transfered value
    value: U256,
    /// Gas
    gas: U256,
    /// Data
    input: Bytes,
    /// Gas price
    gas_price: U256,
    /// The network id of the transaction, if any.
    chain_id: Option<U64>,
    /// The standardised V field of the signature.
    v: U256,
    /// The R field of the signature.
    r: U256,
    /// The S field of the signature.
    s: U256,
    /// EIP-2718 type
    #[serde(rename = "type")]
    transaction_type: U256,
}

impl Transaction {
    pub fn build() -> Self {
        Self {
            nonce: U256::one(),
            from: H160::zero(),
            to: Some(H160::zero()),
            value: U256::one(),
            gas: U256::one(),
            input: Default::default(),
            gas_price: U256::one(),
            chain_id: Some(U64::one()),
            v: U256::one(),
            r: U256::one(),
            s: U256::one(),
            transaction_type: U256::one(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_json_serde() {
        let transaction = Transaction {
            nonce: U256::zero(),
            from: H160::zero(),
            to: Some(H160::zero()),
            value: U256::zero(),
            gas: U256::zero(),
            input: Default::default(),
            gas_price: U256::zero(),
            chain_id: Some(U64::zero()),
            v: U256::zero(),
            r: U256::zero(),
            s: U256::zero(),
            transaction_type: U256::zero(),
        };
        assert_eq!(
            serde_json::to_string(&transaction).unwrap(),
            "{\"nonce\":\"0x0\",\"from\":\"0x0000000000000000000000000000000000000000\",\"to\":\"0x0000000000000000000000000000000000000000\",\"value\":\"0x0\",\"gas\":\"0x0\",\"input\":\"0x\",\"gasPrice\":\"0x0\",\"chainId\":\"0x0\",\"v\":\"0x0\",\"r\":\"0x0\",\"s\":\"0x0\",\"type\":\"0x0\"}"
        );
        let decoded: Transaction = serde_json::from_str("{\"nonce\":\"0x0\",\"from\":\"0x0000000000000000000000000000000000000000\",\"to\":\"0x0000000000000000000000000000000000000000\",\"value\":\"0x0\",\"gas\":\"0x0\",\"input\":\"0x\",\"gasPrice\":\"0x0\",\"chainId\":\"0x0\",\"v\":\"0x0\",\"r\":\"0x0\",\"s\":\"0x0\",\"type\":\"0x0\"}").unwrap();
        assert_eq!(decoded, transaction);
    }
}
