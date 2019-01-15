//! Transaction service abstraction.

use crate::codes;

use chain_core::property;

use futures::prelude::*;

use std::fmt;

/// Interface for the blockchain node service implementation responsible for
/// validating and accepting transactions.
pub trait TransactionService {
    /// The transaction identifier type for the blockchain.
    type TransactionId: property::TransactionId;

    /// The type of asynchronous futures returned by method `propose_transactions`.
    type ProposeTransactionsFuture: Future<
        Item = ProposeTransactionsResponse<Self::TransactionId>,
        Error = TransactionError,
    >;

    /// The type of asynchronous futures returned by method `record_transaction`.
    type RecordTransactionFuture: Future<
        Item = RecordTransactionResponse<Self::TransactionId>,
        Error = TransactionError,
    >;

    /// Given a list of transaction IDs, return status of the transactions
    /// as known by this node.
    fn propose_transactions(
        &mut self,
        ids: &[Self::TransactionId],
    ) -> Self::ProposeTransactionsFuture;
}

/// Represents errors that can be returned by the node service implementation.
#[derive(Debug)]
pub struct TransactionError(); // TODO: define specific error variants and details

impl std::error::Error for TransactionError {}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unknown transaction service error")
    }
}

/// Response from the `propose_transactions` method of a `TransactionService`.
pub struct ProposeTransactionsResponse<Id> {
    // TODO: define fully
    _items: Vec<(Id, codes::TransactionStatus)>,
}

/// Response from the `record_transactions` method of a `TransactionService`.
pub struct RecordTransactionResponse<Id> {
    // TODO: define
    _id: Id,
    _result: codes::TransactionAcceptance,
}
