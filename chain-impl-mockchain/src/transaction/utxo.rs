use super::transaction::TransactionId;
use crate::account;
use crate::key::{
    deserialize_signature, serialize_signature, Hash, SpendingPublicKey, SpendingSecretKey,
    SpendingSignature,
};
use crate::value::*;
use chain_core::property;
use chain_crypto::Verification;

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
