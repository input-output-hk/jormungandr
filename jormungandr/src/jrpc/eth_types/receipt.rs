use super::{log::Log, number::Number};
use chain_evm::ethereum_types::{Bloom, H160, H256};
use serde::Serialize;

/// Receipt
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    /// Transaction Hash
    transaction_hash: H256,
    /// Transaction index
    transaction_index: Number,
    /// Block hash
    block_hash: H256,
    /// Block number
    block_number: Number,
    /// Sender
    from: H160,
    /// Recipient
    to: Option<H160>,
    /// Cumulative gas used
    cumulative_gas_used: Number,
    /// Gas used
    gas_used: Number,
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
    status: Option<Number>,
    /// Effective gas price.
    // Pre-eip1559 this is just the gasprice. Post-eip1559 this is base fee + priority fee.
    effective_gas_price: Number,
}

impl Receipt {
    #[allow(dead_code)]
    pub fn build() -> Self {
        Self {
            transaction_hash: H256::zero(),
            transaction_index: 1.into(),
            block_hash: H256::zero(),
            block_number: 1.into(),
            from: H160::zero(),
            to: Some(H160::zero()),
            cumulative_gas_used: 1.into(),
            gas_used: 1.into(),
            // This should be None if 'to' field has been set and vice versa
            contract_address: None,
            logs: vec![Log::build()],
            logs_bloom: Default::default(),
            root: Some(H256::zero()),
            status: Some(1.into()),
            effective_gas_price: 1.into(),
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
            transaction_index: 0.into(),
            block_hash: H256::zero(),
            block_number: 0.into(),
            from: H160::zero(),
            to: Some(H160::zero()),
            cumulative_gas_used: 0.into(),
            gas_used: 0.into(),
            contract_address: Some(H160::zero()),
            logs: Default::default(),
            logs_bloom: Default::default(),
            root: Some(H256::zero()),
            status: Some(0.into()),
            effective_gas_price: 0.into(),
        };
        assert_eq!(
            serde_json::to_string(&receipt).unwrap(),
            r#"{"transactionHash":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionIndex":"0x0","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000000","blockNumber":"0x0","from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","cumulativeGasUsed":"0x0","gasUsed":"0x0","contractAddress":"0x0000000000000000000000000000000000000000","logs":[],"logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","root":"0x0000000000000000000000000000000000000000000000000000000000000000","status":"0x0","effectiveGasPrice":"0x0"}"#
        );
    }
}
