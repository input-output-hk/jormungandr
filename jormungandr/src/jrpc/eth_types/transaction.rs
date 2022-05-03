use chain_evm::ethereum_types::{H160, U256, U64};
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
    pub fn build() -> Self {
        Self {
            nonce: U256::one(),
            from: H160::zero(),
            to: Some(H160::zero()),
            value: U256::one(),
            gas: U256::one(),
            input: Default::default(),
            gas_price: Some(U256::one()),
            chain_id: Some(U64::one()),
            v: U256::one(),
            r: U256::one(),
            s: U256::one(),
            transaction_type: U256::one(),
        }
    }
}
