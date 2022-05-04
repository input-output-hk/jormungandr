use chain_evm::ethereum_types::{Bloom, H160, H256, U256, U64};
use serde::Serialize;

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
    // TODO use Log type
    logs: Vec<()>,
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
            logs: Default::default(),
            logs_bloom: Default::default(),
            root: Some(H256::zero()),
            status: Some(U64::zero()),
            effective_gas_price: U256::zero(),
        }
    }
}
