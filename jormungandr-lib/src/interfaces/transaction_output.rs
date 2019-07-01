use crate::interfaces::{Address, Value};
use chain_impl_mockchain::transaction::Output;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TransactionOutput {
    address: Address,
    value: Value,
}

impl TransactionOutput {
    /// create a new Transaction output from the given values
    #[inline]
    pub fn new(address: Address, value: Value) -> Self {
        TransactionOutput { address, value }
    }

    /// get the address component of the `TransactionOutput`
    #[inline]
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// get the value component of the `TransactionOutput`
    #[inline]
    pub fn value(&self) -> &Value {
        &self.value
    }
}

/* ---------------- Conversion --------------------------------------------- */

impl From<Output<chain_addr::Address>> for TransactionOutput {
    fn from(v: Output<chain_addr::Address>) -> Self {
        TransactionOutput {
            address: v.address.into(),
            value: v.value.into(),
        }
    }
}

impl From<TransactionOutput> for Output<chain_addr::Address> {
    fn from(v: TransactionOutput) -> Self {
        Output {
            address: v.address.into(),
            value: v.value.into(),
        }
    }
}
