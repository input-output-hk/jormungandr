use chain_evm::ethereum_types::{Bloom, H160, H256, U256, U64};
use serde::Serialize;

use super::log::Log;

/// Receipt
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    /// Transaction Hash
    transaction_hash: H256,
    /// Transaction index
    transaction_index: U256,
    /// Block hash
    block_hash: H256,
    /// Block number
    block_number: U256,
    /// Sender
    from: H160,
    /// Recipient
    to: Option<H160>,
    /// Cumulative gas used
    cumulative_gas_used: U256,
    /// Gas used
    gas_used: U256,
    /// Contract address
    contract_address: Option<H160>,
    /// Logs
    logs: Vec<Log>,
    /// Logs bloom
    logs_bloom: Bloom,
    /// State Root
    // EIP98 makes this optional field, if it's missing then skip serializing it
    root: Option<H256>,
    /// Status code
    // Unknown after EIP98 rules, if it's missing then skip serializing it
    status: Option<U64>,
    /// Effective gas price.
    // Pre-eip1559 this is just the gasprice. Post-eip1559 this is base fee + priority fee.
    effective_gas_price: U256,
}

impl Receipt {
    pub fn build() -> Self {
        Self {
            transaction_hash: H256::zero(),
            transaction_index: U256::zero(),
            block_hash: H256::zero(),
            block_number: U256::zero(),
            from: H160::zero(),
            to: Some(H160::zero()),
            cumulative_gas_used: U256::zero(),
            gas_used: U256::zero(),
            // This should be None if 'to' field has been set and vice versa
            contract_address: None,
            logs: vec![Log::build()],
            logs_bloom: Default::default(),
            root: Some(H256::zero()),
            status: Some(U64::zero()),
            effective_gas_price: U256::zero(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_json_serialize() {
        let receipt = Receipt {
            transaction_hash: H256::zero(),
            transaction_index: U256::zero(),
            block_hash: H256::zero(),
            block_number: U256::zero(),
            from: H160::zero(),
            to: Some(H160::zero()),
            cumulative_gas_used: U256::zero(),
            gas_used: U256::zero(),
            contract_address: Some(H160::zero()),
            logs: Default::default(),
            logs_bloom: Default::default(),
            root: Some(H256::zero()),
            status: Some(U64::zero()),
            effective_gas_price: U256::zero(),
        };
        assert_eq!(
            serde_json::to_string(&receipt).unwrap(),
            r#"{"transactionHash":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionIndex":"0x0","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000000","blockNumber":"0x0","from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","cumulativeGasUsed":"0x0","gasUsed":"0x0","contractAddress":"0x0000000000000000000000000000000000000000","logs":[],"logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","root":"0x0000000000000000000000000000000000000000000000000000000000000000","status":"0x0","effectiveGasPrice":"0x0"}"#
        );
    }
}
