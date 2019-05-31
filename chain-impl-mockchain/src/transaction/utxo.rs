use super::transaction::TransactionId;
use crate::value::*;

pub type TransactionIndex = u8;

/// Unspent transaction pointer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtxoPointer {
    /// the transaction identifier where the unspent output is
    pub transaction_id: TransactionId,
    /// the output index within the pointed transaction's outputs
    pub output_index: TransactionIndex,
    /// the value we expect to read from this output
    ///
    /// This setting is added in order to protect undesired withdrawal
    /// and to set the actual fee in the transaction.
    pub value: Value,
}

impl UtxoPointer {
    pub fn new(
        transaction_id: TransactionId,
        output_index: TransactionIndex,
        value: Value,
    ) -> Self {
        UtxoPointer {
            transaction_id,
            output_index,
            value,
        }
    }
}

impl std::fmt::Display for UtxoPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}@{}.{}",
            self.transaction_id, self.output_index, self.value
        )
    }
}
